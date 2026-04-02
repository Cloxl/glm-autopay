use serde::{Deserialize, Serialize, ser::SerializeMap};

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

/// pay-middle-page 用到的 info 载荷，字段顺序需与官方一致
pub struct PayMiddleInfo {
    pub product_id: String,
    pub product_name: String,
    pub amount: String,
    pub customer_id: String,
    pub customer_name: String,
    pub old_product_id: String,
    pub agreement_no: String,
    pub is_subscribe: bool,
    pub biz_id: String,
    pub pay_type: String,
    pub user_state: String,
    pub ic: String,
}

/// 手动序列化以保证 camelCase 键名顺序与官方前端完全一致
impl Serialize for PayMiddleInfo {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(12))?;
        map.serialize_entry("productId", &self.product_id)?;
        map.serialize_entry("productName", &self.product_name)?;
        map.serialize_entry("amount", &self.amount)?;
        map.serialize_entry("customerId", &self.customer_id)?;
        map.serialize_entry("customerName", &self.customer_name)?;
        map.serialize_entry("oldProductId", &self.old_product_id)?;
        map.serialize_entry("agreementNo", &self.agreement_no)?;
        map.serialize_entry("isSubscribe", &self.is_subscribe)?;
        map.serialize_entry("bizId", &self.biz_id)?;
        map.serialize_entry("payType", &self.pay_type)?;
        map.serialize_entry("userState", &self.user_state)?;
        map.serialize_entry("ic", &self.ic)?;
        map.end()
    }
}

// --- batch-preview 响应 ---

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchPreviewData {
    pub product_list: Option<Vec<ProductInfo>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductInfo {
    pub product_id: String,
    pub pay_amount: Option<f64>,
    pub monthly_pay_amount: Option<f64>,
    pub sold_out: Option<bool>,
}
