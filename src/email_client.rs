use crate::domain::SubscriberEmail;
use reqwest::{Client, Url};
use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;
use std::time::Duration;

pub struct EmailClient {
    client: Client,
    url: Url,
    sender: SubscriberEmail,
    auth_token: SecretString,
}

impl EmailClient {
    pub fn new(
        url: Url,
        sender: SubscriberEmail,
        auth_token: SecretString,
        timeout: Duration,
    ) -> Self {
        let client = Client::builder().timeout(timeout).build().unwrap();

        Self {
            client,
            url,
            sender,
            auth_token,
        }
    }

    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_body: &str,
        text_body: &str,
    ) -> Result<(), reqwest::Error> {
        let body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject,
            text_body,
            html_body,
        };

        let url = self
            .url
            .join("email")
            .expect("Given URL cannot fail parsing.");

        self.client
            .post(url)
            .header("X-Postmark-Server-Token", self.auth_token.expose_secret())
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    text_body: &'a str,
    html_body: &'a str,
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};
    use fake::{
        faker::{
            internet::en::SafeEmail,
            lorem::en::{Paragraph, Sentence},
        },
        Fake, Faker,
    };
    use wiremock::{matchers, Match, Mock, MockServer, Request, ResponseTemplate};

    struct SendEmailBodyMatcher;

    impl Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            let body: serde_json::Value = match request.body_json() {
                Ok(v) => v,
                Err(_) => return false,
            };

            body.get("From").is_some()
                && body.get("To").is_some()
                && body.get("Subject").is_some()
                && body.get("HtmlBody").is_some()
                && body.get("TextBody").is_some()
        }
    }

    /// Generates fake data and sends an email request to the given `MockServer` by using
    /// `EmailClient::send_email`.
    async fn send_fake_email(mock_server: &MockServer) -> Result<(), reqwest::Error> {
        let email_client = {
            let parse = mock_server.uri().parse().unwrap();
            let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
            let auth_token = SecretString::from(Faker.fake::<String>());
            let timeout = Duration::from_millis(200);
            EmailClient::new(parse, sender, auth_token, timeout)
        };

        // Arrange - Generate Fake Data
        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let content: String = Paragraph(1..10).fake();
        let subject: String = Sentence(1..2).fake();

        email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await
    }

    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        // Arrange
        let server = MockServer::start().await;

        Mock::given(matchers::header_exists("X-Postmark-Server-Token"))
            .and(matchers::header("Content-Type", "application/json"))
            .and(matchers::path("/email"))
            .and(matchers::method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        // Act
        let _ = send_fake_email(&server).await;

        // Assert
    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_server_returns_200() {
        // Arrange
        let server = MockServer::start().await;

        Mock::given(matchers::any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        // Act
        let outcome = send_fake_email(&server).await;

        // Assert
        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        // Arrange
        let server = MockServer::start().await;

        Mock::given(matchers::any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&server)
            .await;

        // Act
        let outcome = send_fake_email(&server).await;

        // Assert
        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        // Arrange
        let server = MockServer::start().await;

        let response = ResponseTemplate::new(200).set_delay(Duration::from_secs(180));
        Mock::given(matchers::any())
            .respond_with(response)
            .expect(1)
            .mount(&server)
            .await;

        // Act
        let outcome = send_fake_email(&server).await;

        // Assert
        assert_err!(outcome);
    }
}
