use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub code: Option<i32>,
    pub data: Option<T>,
    pub msg: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRequest<'a> {
    pub product_id: &'a str,
    pub invitation_code: &'a str,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewData {
    pub biz_id: Option<String>,
    pub sold_out: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSignRequest<'a> {
    pub pay_type: &'a str,
    pub product_id: &'a str,
    pub customer_id: &'a str,
    pub biz_id: &'a str,
    pub invitation_code: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct CreateSignData {
    pub sign: Option<String>,
}
