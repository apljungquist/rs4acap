//! Facilities for getting the [`AxisConnectSessionSID`] used to authenticate with the VLT.
//!
//! The authentication flow starts with [`AuthenticationFlow::start`].

// The flow has been inferred from what the web app does and is not based on a public API.
// As such it may be prone to breaking in more or less obvious ways.
// TODO: Consider embedding more self checks (form actions, etc)

use std::{collections::HashMap, fmt::Formatter, sync::Arc};

use anyhow::{bail, Context};
use base32::Alphabet;
use log::info;
use regex::Regex;
use reqwest::{
    cookie::{CookieStore, Jar},
    Client,
};
use url::Url;

fn match_axis_auth_cookie(html: &str) -> anyhow::Result<String> {
    let re = Regex::new(r#"cookieHandler\.setSessionCookie\(\s*["']axis_auth["']\s*,\s*["'](?<value>[^"']+)["'],\s*(?<needsEncoding>false|true)\)"#).expect("Literal is valid regex");
    let m = re.captures(html).context("Failed to find cookie")?;
    debug_assert_eq!(
        m.name("needsEncoding")
            .expect("Pattern captures this name")
            .as_str(),
        "false"
    );
    Ok(m.name("value")
        .expect("Pattern captures this name")
        .as_str()
        .to_string())
}

// TODO: Consider parsing the html as intermediate step to make this more robust
fn match_state_input(html: &str) -> anyhow::Result<String> {
    let re = Regex::new(r#"<input type="hidden" name="state" value="([^"]*)"/>"#)
        .expect("Literal is valid regex");
    re.captures(html).context("Failed to find state").map(|m| {
        m.get(1)
            .expect("Pattern captures at least one group")
            .as_str()
            .to_string()
    })
}

fn match_token_input(html: &str) -> anyhow::Result<String> {
    let re = Regex::new(r#"<input type="hidden" name="token" value="([^"]*)"/>"#)
        .expect("Literal is valid regex");
    re.captures(html).context("Failed to find token").map(|m| {
        m.get(1)
            .expect("Pattern captures at least one group")
            .as_str()
            .to_string()
    })
}

/// Adapter for the username and password form.
///
/// Public only to make it possible to use as the state for [`AuthenticationFlow`].
pub struct UsernamePasswordForm;

impl UsernamePasswordForm {
    async fn get(client: &Client) -> anyhow::Result<Self> {
        info!("Getting {} ...", std::any::type_name::<Self>());

        let text = client
            .get("https://www.axis.com/my-axis/login")
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        Self::try_from_str(&text)
    }

    fn try_from_str(_: &str) -> anyhow::Result<Self> {
        Ok(Self)
    }

    async fn submit(
        self,
        client: &Client,
        username: &str,
        password: &str,
    ) -> anyhow::Result<MethodSelectionForm> {
        info!("Submitting {} ...", std::any::type_name::<Self>());

        let encoded = base32::encode(Alphabet::Rfc4648 { padding: true }, password.as_bytes());

        let mut form_data = HashMap::new();
        form_data.insert("userName", username);
        form_data.insert("passwordfield", password);
        form_data.insert("password", &encoded);

        let text = client
            .post("https://auth.axis.com/authn/authentication/html")
            .form(&form_data)
            .send()
            .await
            .context("Failed to submit username and password form")?
            .error_for_status()?
            .text()
            .await?;

        MethodSelectionForm::try_from_str(&text)
    }
}

struct MethodSelectionForm;

impl MethodSelectionForm {
    fn try_from_str(_: &str) -> anyhow::Result<Self> {
        Ok(Self)
    }
    async fn submit(self, client: &Client) -> anyhow::Result<OneTimePasswordForm> {
        info!("Submitting {} ...", std::any::type_name::<Self>());

        let mut form_data = HashMap::new();
        form_data.insert("acr", "urn:se:curity:authentication:email:email-wcode");
        form_data.insert("method_0", "email-wcode");

        let text = client
            .post("https://auth.axis.com/authn/authentication/_action/opt-in/select")
            .form(&form_data)
            .send()
            .await
            .context("Failed to submit otp form")?
            .error_for_status()?
            .text()
            .await?;

        OneTimePasswordForm::try_from_str(&text)
    }
}

/// Adapter for the one-time password form.
///
/// Public only to make it possible to use as the state for [`AuthenticationFlow`].
pub struct OneTimePasswordForm;

impl OneTimePasswordForm {
    fn try_from_str(_: &str) -> anyhow::Result<Self> {
        Ok(Self)
    }
    async fn submit(self, client: &Client, otp: &str) -> anyhow::Result<EmptyForm> {
        info!("Submitting {} ...", std::any::type_name::<Self>());

        let mut form_data = HashMap::new();
        form_data.insert("otp", otp);

        let text = client
            .post("https://auth.axis.com/authn/authentication/email-wcode/enter-otp")
            .form(&form_data)
            .send()
            .await
            .context("Failed to submit otp form")?
            .error_for_status()?
            .text()
            .await?;

        EmptyForm::try_from_str(&text)
    }
}

struct EmptyForm {
    axis_auth: String,
}

impl EmptyForm {
    fn try_from_str(s: &str) -> anyhow::Result<Self> {
        let axis_auth = match_axis_auth_cookie(s)?;

        Ok(Self { axis_auth })
    }
    async fn submit(self, client: &Client, jar: &Jar) -> anyhow::Result<StateTokenForm> {
        info!("Submitting {} ...", std::any::type_name::<Self>());

        let cookie = format!("axis_auth={}", self.axis_auth);
        jar.add_cookie_str(
            &cookie,
            &Url::parse("https://axis.com").expect("Literal is valid URL"),
        );

        let text = client
            .post("https://auth.axis.com/authn/authentication/_action/axis-cookie")
            .send()
            .await?
            .text()
            .await?;

        StateTokenForm::try_from_str(&text)
    }
}

struct StateTokenForm {
    state: String,
    token: String,
}

impl StateTokenForm {
    fn try_from_str(s: &str) -> anyhow::Result<Self> {
        let state = match_state_input(s)?;
        let token = match_token_input(s)?;

        Ok(Self { state, token })
    }

    async fn submit(self, client: &Client, jar: &Jar) -> anyhow::Result<AxisConnectSessionSID> {
        info!("Submitting {} ...", std::any::type_name::<Self>());

        let mut form_data = HashMap::new();
        form_data.insert("state", self.state);
        form_data.insert("token", self.token);

        // This redirects to an HTML page with no information of interest.
        client
            .post("https://auth.axis.com/oauth2/oauth-authorize")
            .form(&form_data)
            .send()
            .await
            .context("Failed to submit form")?
            .error_for_status()?
            .text()
            .await?;

        let url = Url::parse("https://axis.com").expect("Literal is valid URL");
        let cookies = jar.cookies(&url).context("No cookies found for url")?;
        let mut cookies = cookies
            .to_str()?
            .split("; ")
            .filter_map(AxisConnectSessionSID::from_str);
        let Some(cookie) = cookies.next() else {
            bail!("No cookie found");
        };
        if cookies.next().is_some() {
            bail!("More than one cookie found");
        }
        Ok(cookie)
    }
}

// This cookie appears to be sufficient for most VLT APIs, but when used with the `getAll` API the
// response is always a failure. I think this means the authentication was accepted, but I cannot
// seem to figure out how to make that API return success. Replaying successful requests from the
// browser is one of many things that don't work.
// TODO: Figure out how the `getAll` API works

const SID_COOKIE_PREFIX: &str = "axis_connect_session_sid=";

/// The cookie that grants access to most VLT APIs
pub struct AxisConnectSessionSID(pub(crate) String);

impl AxisConnectSessionSID {
    pub fn try_from_string(s: String) -> anyhow::Result<Self> {
        if s.starts_with(SID_COOKIE_PREFIX) {
            Ok(Self(s))
        } else {
            bail!("Invalid cookie: {}", s);
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        if s.starts_with(SID_COOKIE_PREFIX) {
            Some(Self(s.to_string()))
        } else {
            None
        }
    }
}

impl std::fmt::Display for AxisConnectSessionSID {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// State management for the authentication flow.
pub struct AuthenticationFlow<T> {
    client: Client,
    jar: Arc<Jar>,
    form: T,
}

impl AuthenticationFlow<UsernamePasswordForm> {
    /// Start a new authentication flow using username, password and an OTP sent over email.
    pub async fn start() -> anyhow::Result<Self> {
        let jar = Arc::new(Jar::default());
        let client = Client::builder()
            .cookie_provider(Arc::clone(&jar))
            .redirect(reqwest::redirect::Policy::default())
            .build()?;

        let form = UsernamePasswordForm::get(&client).await?;

        Ok(Self { client, jar, form })
    }

    // TODO: Consider detecting bad input and allow user to retry
    /// Submit the username and password.
    pub async fn submit(
        self,
        username: &str,
        password: &str,
    ) -> anyhow::Result<AuthenticationFlow<OneTimePasswordForm>> {
        let Self { client, jar, form } = self;

        let form = form
            .submit(&client, username, password)
            .await?
            .submit(&client)
            .await?;
        Ok(AuthenticationFlow { client, jar, form })
    }
}

impl AuthenticationFlow<OneTimePasswordForm> {
    // TODO: Consider detecting bad input and allow user to retry
    /// Submit the one-time password.
    pub async fn submit(self, otp: &str) -> anyhow::Result<AxisConnectSessionSID> {
        let Self { client, jar, form } = self;

        let sid = form
            .submit(&client, otp)
            .await?
            .submit(&client, &jar)
            .await?
            .submit(&client, &jar)
            .await?;

        Ok(sid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn set_session_cookie_regex_works_on_example() {
        let html = r#">cookieHandler.setSessionCookie("axis_auth", "u%3D123", false);<"#;
        assert_eq!(match_axis_auth_cookie(html).unwrap(), "u%3D123");
    }

    #[test]
    fn state_input_regex_works_on_example() {
        let html = r#"><input type="hidden" name="state" value="R_pBZ"/><"#;
        assert_eq!(match_state_input(html).unwrap(), "R_pBZ");
    }

    #[test]
    fn token_input_regex_works_on_example() {
        let html = r#"><input type="hidden" name="token" value="zVPN"/><"#;
        assert_eq!(match_token_input(html).unwrap(), "zVPN");
    }
}
