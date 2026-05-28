use bcrypt::{hash, verify, DEFAULT_COST};

/// Băm mật khẩu trần bằng thuật toán bcrypt cực kỳ bảo mật
pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    hash(password, DEFAULT_COST)
}

/// Kiểm tra mật khẩu trần khớp với chuỗi mật khẩu đã băm hay không
pub fn verify_password(password: &str, hashed: &str) -> Result<bool, bcrypt::BcryptError> {
    verify(password, hashed)
}
