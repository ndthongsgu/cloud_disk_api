use std::fmt;
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};

/// Lỗi xảy ra khi tương tác với API của Telegram
#[derive(Debug)]
pub enum TelegramError {
    /// Lỗi mạng hoặc kết nối HTTP
    Network(reqwest::Error),
    /// Lỗi trả về từ API Telegram (ok: false)
    Api {
        code: i32,
        description: String,
    },
    /// Lỗi vượt quá tần suất yêu cầu (Rate Limit - HTTP 429)
    RateLimited {
        retry_after_seconds: u64,
        description: String,
    },
    /// Phản hồi từ Telegram không đúng định dạng mong đợi
    InvalidResponse(String),
    /// Cấu hình Client hoặc Proxy bị lỗi
    Config(String),
}

impl std::error::Error for TelegramError {}

impl fmt::Display for TelegramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TelegramError::Network(e) => write!(f, "Lỗi kết nối mạng: {}", e),
            TelegramError::Api { code, description } => {
                write!(f, "Lỗi Telegram API (Code: {}): {}", code, description)
            }
            TelegramError::RateLimited { retry_after_seconds, description } => {
                write!(f, "Bị Rate Limit Telegram. Vui lòng thử lại sau {} giây. Chi tiết: {}", retry_after_seconds, description)
            }
            TelegramError::InvalidResponse(e) => {
                write!(f, "Phản hồi Telegram không hợp lệ: {}", e)
            }
            TelegramError::Config(e) => write!(f, "Lỗi cấu hình Telegram Client: {}", e),
        }
    }
}

/// Cấu trúc bao gói phản hồi chuẩn của Telegram API
#[derive(Debug, Deserialize)]
struct TelegramResponse<T> {
    ok: bool,
    result: Option<T>,
    description: Option<String>,
    error_code: Option<i32>,
    parameters: Option<TelegramResponseParameters>,
}

#[derive(Debug, Deserialize)]
struct TelegramResponseParameters {
    retry_after: Option<u64>,
}

/// Thông tin người dùng/bot từ API Telegram
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramUser {
    pub id: i64,
    pub is_bot: bool,
    pub first_name: String,
    pub username: Option<String>,
}

/// Thông tin phòng chat/kênh lưu trữ trên Telegram
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramChat {
    pub id: i64,
    pub title: Option<String>,
    pub username: Option<String>,
    #[serde(rename = "type")]
    pub chat_type: String,
}

/// Siêu dữ liệu của File Document được lưu trữ trên Telegram
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramDocument {
    pub file_id: String,
    pub file_unique_id: String,
    pub file_name: Option<String>,
    pub mime_type: Option<String>,
    pub file_size: Option<i64>,
}

/// Thông tin tin nhắn trả về sau khi tải lên thành công
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramMessage {
    pub message_id: i64,
    pub chat: TelegramChat,
    pub date: i64,
    pub document: Option<TelegramDocument>,
}

/// Chi tiết file vật lý trả về từ /getFile phục vụ tải xuống
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramFile {
    pub file_id: String,
    pub file_unique_id: String,
    pub file_size: Option<i64>,
    pub file_path: Option<String>,
}

/// Repository tích hợp và tương tác trực tiếp với Telegram Bot API
#[derive(Clone)]
pub struct TelegramService {
    client: reqwest::Client,
}

impl TelegramService {
    /// Khởi tạo một `TelegramService` mới, tùy chọn cấu hình Proxy (Socks5/Http)
    pub fn new(proxy_url: Option<String>) -> Result<Self, TelegramError> {
        let mut builder = reqwest::Client::builder();

        if let Some(proxy) = proxy_url {
            if !proxy.trim().is_empty() {
                let http_proxy = reqwest::Proxy::all(&proxy)
                    .map_err(|e| TelegramError::Config(format!("Proxy không hợp lệ: {}", e)))?;
                builder = builder.proxy(http_proxy);
            }
        }

        let client = builder
            .build()
            .map_err(TelegramError::Network)?;

        Ok(Self { client })
    }

    /// Helper xử lý và parse kết quả phản hồi của Telegram API
    async fn parse_response<T>(&self, response: reqwest::Response) -> Result<T, TelegramError>
    where
        T: serde::de::DeserializeOwned,
    {
        let status = response.status();
        let text = response.text().await.map_err(TelegramError::Network)?;

        let api_res: TelegramResponse<T> = match serde_json::from_str(&text) {
            Ok(res) => res,
            Err(e) => {
                return Err(TelegramError::InvalidResponse(format!(
                    "Không thể phân tích cú pháp JSON phản hồi: {}. Nội dung thô: {}",
                    e, text
                )));
            }
        };

        if api_res.ok {
            if let Some(result) = api_res.result {
                Ok(result)
            } else {
                Err(TelegramError::InvalidResponse(
                    "Telegram trả về OK nhưng bị thiếu trường 'result' trong JSON".to_string(),
                ))
            }
        } else {
            let code = api_res.error_code.unwrap_or_else(|| status.as_u16() as i32);
            let description = api_res.description.unwrap_or_else(|| "Lỗi không xác định từ Telegram API".to_string());

            if code == 429 {
                let retry_after = api_res
                    .parameters
                    .as_ref()
                    .and_then(|p| p.retry_after)
                    .unwrap_or(5); // Mặc định chờ 5 giây nếu không có thông tin cụ thể
                Err(TelegramError::RateLimited {
                    retry_after_seconds: retry_after,
                    description,
                })
            } else {
                Err(TelegramError::Api { code, description })
            }
        }
    }

    /// Xác minh tính hợp lệ của Bot Token bằng cách lấy thông tin Bot (/getMe)
    pub async fn get_me(&self, bot_token: &str) -> Result<TelegramUser, TelegramError> {
        let url = format!("https://api.telegram.org/bot{}/getMe", bot_token);
        
        let response = self.client.get(&url)
            .send()
            .await
            .map_err(TelegramError::Network)?;

        self.parse_response(response).await
    }

    /// Tải lên một mảnh dữ liệu nhị phân (chunk file) dưới dạng Document lên Telegram
    ///
    /// # Arguments
    /// * `bot_token` - Token API của Telegram Bot thực hiện gửi
    /// * `chat_id` - ID hoặc Username của Kênh/Nhóm/User nhận tệp tin (ví dụ: "-100123456789")
    /// * `file_name` - Tên hiển thị của mảnh file trên Telegram (ví dụ: "part_0.chk")
    /// * `file_bytes` - Dữ liệu nhị phân của mảnh file
    pub async fn send_document(
        &self,
        bot_token: &str,
        chat_id: &str,
        file_name: &str,
        file_bytes: Vec<u8>,
    ) -> Result<TelegramMessage, TelegramError> {
        let url = format!("https://api.telegram.org/bot{}/sendDocument", bot_token);

        let part = Part::bytes(file_bytes)
            .file_name(file_name.to_string());

        let form = Form::new()
            .text("chat_id", chat_id.to_string())
            .part("document", part);

        let response = self.client.post(&url)
            .multipart(form)
            .send()
            .await
            .map_err(TelegramError::Network)?;

        self.parse_response(response).await
    }

    /// Lấy đường dẫn file vật lý trên máy chủ Telegram thông qua file_id (/getFile)
    pub async fn get_file(&self, bot_token: &str, file_id: &str) -> Result<TelegramFile, TelegramError> {
        let url = format!("https://api.telegram.org/bot{}/getFile", bot_token);

        let response = self.client.get(&url)
            .query(&[("file_id", file_id)])
            .send()
            .await
            .map_err(TelegramError::Network)?;

        self.parse_response(response).await
    }

    /// Tải nội dung nhị phân (binary content) của file về từ máy chủ Telegram dựa trên `file_path`
    pub async fn download_file(&self, bot_token: &str, file_path: &str) -> Result<bytes::Bytes, TelegramError> {
        let url = format!("https://api.telegram.org/file/bot{}/{}", bot_token, file_path);

        let response = self.client.get(&url)
            .send()
            .await
            .map_err(TelegramError::Network)?;

        if !response.status().is_success() {
            return Err(TelegramError::Api {
                code: response.status().as_u16() as i32,
                description: format!("Không thể tải file: Mã trạng thái HTTP {}", response.status()),
            });
        }

        let bytes = response.bytes().await.map_err(TelegramError::Network)?;
        Ok(bytes)
    }

    /// Xóa tin nhắn (mảnh file vật lý đã lưu) trên Telegram (/deleteMessage)
    ///
    /// # Arguments
    /// * `bot_token` - Token của Bot
    /// * `chat_id` - ID của Kênh/Trò chuyện chứa tin nhắn
    /// * `message_id` - ID của tin nhắn cần xóa
    pub async fn delete_message(
        &self,
        bot_token: &str,
        chat_id: &str,
        message_id: i64,
    ) -> Result<bool, TelegramError> {
        let url = format!("https://api.telegram.org/bot{}/deleteMessage", bot_token);

        #[derive(Serialize)]
        struct DeleteParams<'a> {
            chat_id: &'a str,
            message_id: i64,
        }

        let params = DeleteParams { chat_id, message_id };

        let response = self.client.post(&url)
            .json(&params)
            .send()
            .await
            .map_err(TelegramError::Network)?;

        self.parse_response(response).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_service() {
        let service = TelegramService::new(None);
        assert!(service.is_ok());

        let service_with_proxy = TelegramService::new(Some("http://127.0.0.1:1080".to_string()));
        assert!(service_with_proxy.is_ok());
    }

    #[test]
    fn test_deserialize_error_response() {
        let err_json = r#"{
            "ok": false,
            "error_code": 400,
            "description": "Bad Request: chat not found"
        }"#;

        let res: Result<TelegramResponse<TelegramUser>, _> = serde_json::from_str(err_json);
        assert!(res.is_ok());
        let parsed = res.unwrap();
        assert!(!parsed.ok);
        assert_eq!(parsed.error_code, Some(400));
        assert_eq!(parsed.description.unwrap(), "Bad Request: chat not found");
    }

    #[test]
    fn test_deserialize_rate_limit_response() {
        let err_json = r#"{
            "ok": false,
            "error_code": 429,
            "description": "Too Many Requests: retry after 15",
            "parameters": {
                "retry_after": 15
            }
        }"#;

        let res: Result<TelegramResponse<TelegramUser>, _> = serde_json::from_str(err_json);
        assert!(res.is_ok());
        let parsed = res.unwrap();
        assert!(!parsed.ok);
        assert_eq!(parsed.error_code, Some(429));
        assert_eq!(parsed.parameters.unwrap().retry_after, Some(15));
    }
}
