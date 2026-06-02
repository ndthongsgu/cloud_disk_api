-- =============================================================================
-- SEED DATA KHỞI TẠO TÀI KHOẢN ADMIN CHO HỆ THỐNG CLOUD DISK
-- =============================================================================

-- Khởi tạo duy nhất tài khoản Quản trị viên tối cao (System Administrator)
-- Tên đăng nhập: admin
-- Mật khẩu đăng nhập: admin
-- Thuật toán băm: bcrypt (cost 12 - DEFAULT_COST) -> $2b$12$iUekldVYk0Zxem2aEw7EE.8QwZhsXcUJ.t1U/CpFyc83bozab5D.K
INSERT INTO users (
    id, 
    created_date, 
    created_by, 
    updated_date, 
    updated_by, 
    version, 
    is_deleted, 
    is_admin, 
    username, 
    password_hash, 
    status, 
    code, 
    full_name, 
    email, 
    phone, 
    department_name, 
    storage_quota_bytes, 
    used_storage_bytes
)
VALUES 
(
    'c0eebc99-9c0b-4ef8-bb6d-6bb9bd380a13', 
    CURRENT_TIMESTAMP, 
    NULL, 
    CURRENT_TIMESTAMP, 
    NULL, 
    1, 
    false, 
    true, -- Chỉ cần cờ is_admin = true để kích hoạt đặc quyền Admin
    'admin', 
    '$2b$12$iUekldVYk0Zxem2aEw7EE.8QwZhsXcUJ.t1U/CpFyc83bozab5D.K', -- Chuỗi hash bcrypt cost 12 cho mật khẩu "123456"
    'ACTIVE', 
    'USER-ADMIN', 
    'Admin', 
    'admin@clouddisk.com', 
    '0123456789', 
    'Cloud Disk', 
    0, 
    0
)
ON CONFLICT (username) DO NOTHING;
