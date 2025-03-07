use crate::helpers::{self, TestApp};

#[tokio::test]
async fn you_must_be_logged_in_to_access_the_admin_dashboard() {
    // Arrange
    let app = TestApp::spawn().await;

    // Act
    let resp = app.get_admin_dashboard().await;

    // Assert
    helpers::assert_redirecting(&resp, "/login");
}
