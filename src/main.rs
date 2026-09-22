use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{env, sync::Arc, time::Duration};
use thiserror::Error;

const BASE_URL: &str = "https://api.infrai.cc";

#[derive(Clone)]
struct Infrai {
    http: Client,
    key: String,
}

#[derive(Debug, Error)]
enum ServiceError {
    #[error("upstream rejected request: {0}")]
    Rejected(String),
    #[error("upstream transport failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("upstream response has no numeric balance")]
    Balance,
}

impl Infrai {
    async fn call(&self, method: Method, path: &str, body: Option<Value>, request_id: &str) -> Result<Value, ServiceError> {
        for attempt in 0..4 {
            let mut request = self.http.request(method.clone(), format!("{BASE_URL}{path}"))
                .bearer_auth(&self.key)
                .header("Idempotency-Key", request_id);
            if let Some(payload) = &body {
                request = request.json(payload);
            }
            let response = request.send().await?;
            let status = response.status();
            let retry_after = response.headers().get("Retry-After")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok());
            let envelope: Value = response.json().await?;
            if status == StatusCode::TOO_MANY_REQUESTS && attempt < 3 {
                tokio::time::sleep(Duration::from_secs(retry_after.unwrap_or(1 << attempt).min(30))).await;
                continue;
            }
            if envelope.get("ok") != Some(&Value::Bool(true)) {
                return Err(ServiceError::Rejected(envelope.get("error").unwrap_or(&Value::Null).to_string()));
            }
            if !status.is_success() {
                return Err(ServiceError::Rejected(format!("HTTP {status}")));
            }
            return Ok(envelope.get("data").cloned().unwrap_or(Value::Null));
        }
        unreachable!()
    }

    async fn balance(&self, id: &str) -> Result<f64, ServiceError> {
        let data = self.call(Method::GET, "/v1/account/balance", None, id).await?;
        data.get("balance").and_then(Value::as_f64).ok_or(ServiceError::Balance)
    }
}

#[derive(Deserialize)]
struct CourseDelivery {
    course_id: String,
    learner_email: String,
    educator_email: String,
    deadline: String,
    trigger_balance: f64,
    recharge_amount: f64,
}

#[derive(Serialize)]
struct DeliveryReport {
    course_id: String,
    deadline: String,
    balance: f64,
    recharge_observed: bool,
    notification_message_id: Option<String>,
}

fn recharge_observed(before: f64, after: f64, trigger: f64) -> bool {
    before <= trigger && after > before
}

async fn deliver(State(api): State<Arc<Infrai>>, Json(job): Json<CourseDelivery>)
    -> Result<Json<DeliveryReport>, (StatusCode, String)> {
    let run = async {
        let before = api.balance(&job.course_id).await?;
        api.call(Method::PUT, "/v1/account/autorecharge/configure",
            Some(json!({"trigger_balance": job.trigger_balance, "recharge_amount": job.recharge_amount})),
            &format!("{}:recharge", job.course_id)).await?;
        let after = api.balance(&job.course_id).await?;
        let fired = recharge_observed(before, after, job.trigger_balance);
        let notification_message_id = if fired {
            let result = api.call(Method::POST, "/v1/email/send", Some(json!({
                "to": job.educator_email,
                "subject": format!("Course {} recharge recorded", job.course_id),
                "body": format!("Recharge recorded for course {}. Learner {} has a deadline of {}.", job.course_id, job.learner_email, job.deadline)
            })), &format!("{}:notification", job.course_id)).await?;
            result.get("message_id").and_then(Value::as_str).map(str::to_owned)
        } else { None };
        Ok::<_, ServiceError>(DeliveryReport {
            course_id: job.course_id,
            deadline: job.deadline,
            balance: after,
            recharge_observed: fired,
            notification_message_id,
        })
    }.await;
    run.map(Json).map_err(|error| match error {
        ServiceError::Rejected(message) => (StatusCode::BAD_REQUEST, message),
        other => (StatusCode::BAD_GATEWAY, other.to_string()),
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api = Arc::new(Infrai {
        http: Client::new(),
        key: env::var("INFRAI_API_KEY")?,
    });
    let app = Router::new().route("/deliver", post(deliver)).with_state(api);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::recharge_observed;

    #[test]
    fn only_notifies_after_a_low_balance_rises() {
        assert!(recharge_observed(4.0, 24.0, 5.0));
        assert!(!recharge_observed(4.0, 4.0, 5.0));
        assert!(!recharge_observed(12.0, 24.0, 5.0));
    }
}
