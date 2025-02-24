use crate::domain::SubscriberEmail;
use reqwest::{Client, Url};
use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;

pub struct EmailClient {
    client: Client,
    url: Url,
    sender: SubscriberEmail,
    auth_token: SecretString,
}

impl EmailClient {
    pub fn new(url: Url, sender: SubscriberEmail, auth_token: SecretString) -> Self {
        Self {
            client: Client::new(),
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
    ) -> Result<(), String> {
        let body = SendEmailRequest {
            from: &self.sender.as_ref(),
            to: recipient.as_ref(),
            subject,
            text_body,
            html_body,
        };

        let _rqst = {
            let url = self
                .url
                .join("email")
                .expect("Given URL cannot fail parsing.");

            self.client
                .post(url)
                .header("X-Postmark-Server-Token", self.auth_token.expose_secret())
                .json(&body)
                .send()
                .await
        };

        Ok(())
    }
}

#[derive(Serialize)]
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
    use fake::{
        faker::{
            internet::en::SafeEmail,
            lorem::en::{Paragraph, Sentence},
        },
        Fake, Faker,
    };
    use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn send_email_fires_a_request_to_base_url() {
        // Arrange
        let mock_server = {
            let server = MockServer::start().await;
            Mock::given(matchers::any())
                .respond_with(ResponseTemplate::new(200))
                .expect(1)
                .mount(&server)
                .await;
            server
        };

        let email_client = {
            let parse = mock_server.uri().parse().unwrap();
            let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
            let auth_token = SecretString::from(Faker.fake::<String>());
            EmailClient::new(parse, sender, auth_token)
        };

        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let content: String = Paragraph(1..10).fake();
        let subject: String = Sentence(1..2).fake();

        // Act
        let _ = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;

        // Assert
    }
}
