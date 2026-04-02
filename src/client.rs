use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use serde::{de::DeserializeOwned, Serialize};
use wreq::{
    header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE, COOKIE, ORIGIN, REFERER, USER_AGENT},
    Client,
};
use wreq_util::Emulation;

#[derive(Clone)]
pub struct ApiClient {
    inner: Client,
    #[allow(dead_code)]
    pub config: AppConfig,
}

impl ApiClient {
    pub fn new(config: AppConfig) -> AppResult<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, header_value(&config.user_agent)?);
        headers.insert(ACCEPT, HeaderValue::from_static("application/json, text/plain, */*"));
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/json;charset=UTF-8"),
        );
        headers.insert(COOKIE, header_value(&config.cookie)?);
        headers.insert("authorization", header_value(&config.auth_token)?);
        headers.insert("bigmodel-organization", header_value(&config.bigmodel_organization)?);
        headers.insert("bigmodel-project", header_value(&config.bigmodel_project)?);
        headers.insert(ORIGIN, HeaderValue::from_static("https://www.bigmodel.cn"));
        headers.insert(REFERER, HeaderValue::from_static("https://www.bigmodel.cn/glm-coding"));

        let inner = Client::builder()
            .default_headers(headers)
            .emulation(Emulation::Chrome137)
            .build()?;

        Ok(Self { inner, config })
    }

    #[allow(dead_code)]
    pub async fn post_text<T: Serialize + ?Sized>(&self, url: &str, body: &T) -> AppResult<String> {
        let text = self.inner.post(url).json(body).send().await?.text().await?;
        Ok(text)
    }

    pub async fn post_json<T: Serialize + ?Sized, R: DeserializeOwned>(
        &self,
        url: &str,
        body: &T,
    ) -> AppResult<R> {
        let value = self.inner.post(url).json(body).send().await?.json::<R>().await?;
        Ok(value)
    }
}

fn header_value(value: &str) -> AppResult<HeaderValue> {
    HeaderValue::from_str(value).map_err(|_| AppError::InvalidHeader(value.to_string()))
}
