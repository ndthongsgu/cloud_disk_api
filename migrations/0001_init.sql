-- =============================================================================
-- KHỞI TẠO CƠ SỞ DỮ LIỆU HỆ THỐNG CLOUD DISK (LƯU TRỮ ĐÁM MÂY TELEGRAM BOT)
-- =============================================================================

-- -----------------------------------------------------------------------------
-- 1. BẢNG users: Quản lý thông tin tài khoản và đăng nhập của hệ thống
-- -----------------------------------------------------------------------------
CREATE TABLE users (
    id UUID PRIMARY KEY,                                     -- ID duy nhất của tài khoản (UUID)
    created_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, -- Thời gian tạo tài khoản
    created_by UUID,                                         -- ID người tạo tài khoản
    updated_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, -- Thời gian cập nhật tài khoản gần nhất
    updated_by UUID,                                         -- ID người cập nhật tài khoản gần nhất
    version INT NOT NULL DEFAULT 1,                          -- Phiên bản bản ghi (tránh xung đột ghi đồng thời)
    is_deleted BOOLEAN NOT NULL DEFAULT FALSE,               -- Trạng thái xóa mềm (Soft delete)
    is_admin BOOLEAN NOT NULL DEFAULT FALSE,                  -- Tài khoản admin khởi tạo tự động.
    username VARCHAR(255) UNIQUE NOT NULL,                   -- Tên đăng nhập duy nhất (ví dụ: admin, user1)
    password_hash VARCHAR(255) NOT NULL,                     -- Mã băm mật khẩu bảo mật (Argon2, bcrypt...)
    status VARCHAR(50) NOT NULL,                             -- Trạng thái tài khoản (ACTIVE, LOCKED, BANNED)
    code VARCHAR(100) UNIQUE NOT NULL,                       -- Mã định danh người dùng duy nhất
    full_name VARCHAR(255) NOT NULL,                         -- Họ và tên đầy đủ
    email VARCHAR(255) UNIQUE NOT NULL,                      -- Địa chỉ email duy nhất
    phone VARCHAR(50),                                       -- Số điện thoại liên hệ
    department_name VARCHAR(255),                            -- Tên phòng ban làm việc (nếu là nhân viên)
    storage_quota_bytes BIGINT,                              -- Hạn mức lưu trữ tối đa (NULL = vô hạn)
    used_storage_bytes BIGINT NOT NULL DEFAULT 0             -- Tổng dung lượng đã dùng thực tế (byte)
);

-- -----------------------------------------------------------------------------
-- 2. BẢNG roles: Quản lý danh sách vai trò hệ thống phục vụ RBAC
-- -----------------------------------------------------------------------------
CREATE TABLE roles (
    id UUID PRIMARY KEY,                                     -- ID duy nhất của vai trò
    created_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, -- Thời gian tạo vai trò
    created_by UUID,                                         -- ID người tạo vai trò
    updated_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, -- Thời gian cập nhật vai trò gần nhất
    updated_by UUID,                                         -- ID người cập nhật gần nhất
    version INT NOT NULL DEFAULT 1,                          -- Phiên bản vai trò
    is_deleted BOOLEAN NOT NULL DEFAULT FALSE,               -- Trạng thái xóa mềm
    code VARCHAR(100) UNIQUE NOT NULL,                       -- Mã vai trò duy nhất (ví dụ: ROLE_ADMIN)
    name VARCHAR(255) UNIQUE NOT NULL,                       -- Tên vai trò duy nhất (ví dụ: ADMIN, CUSTOMER)
    description VARCHAR(500),                                -- Mô tả chi tiết vai trò
    permissions TEXT NOT NULL DEFAULT '{}'                   -- Danh sách các quyền hạn dạng JSON string
);


-- -----------------------------------------------------------------------------
-- 3. BẢNG user_roles: Bảng liên kết nhiều-nhiều giữa Users và Roles
-- -----------------------------------------------------------------------------
CREATE TABLE user_roles (
    id UUID PRIMARY KEY,                                     -- ID bản ghi liên kết
    created_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, -- Thời gian liên kết
    created_by UUID,                                         -- Người thực hiện liên kết
    updated_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, -- Thời gian cập nhật liên kết
    updated_by UUID,                                         -- Người cập nhật liên kết
    version INT NOT NULL DEFAULT 1,                          -- Phiên bản bản ghi
    is_deleted BOOLEAN NOT NULL DEFAULT FALSE,               -- Trạng thái xóa mềm
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE, -- Khóa ngoại trỏ đến users(id)
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE, -- Khóa ngoại trỏ đến roles(id)
    CONSTRAINT unique_user_role UNIQUE (user_id, role_id)    -- Ràng buộc duy nhất tránh trùng lặp
);

-- -----------------------------------------------------------------------------
-- 4. BẢNG telegram_bots: Quản lý danh sách các Telegram Bot làm node lưu trữ vật lý
-- -----------------------------------------------------------------------------
CREATE TABLE telegram_bots (
    id UUID PRIMARY KEY,                                     -- ID duy nhất của Bot
    created_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, -- Thời gian thêm Bot vào pool
    created_by UUID,                                         -- Người thêm Bot
    updated_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, -- Thời gian cập nhật trạng thái Bot gần nhất
    updated_by UUID,                                         -- Người cập nhật trạng thái gần nhất
    version INT NOT NULL DEFAULT 1,                          -- Phiên bản bản ghi
    is_deleted BOOLEAN NOT NULL DEFAULT FALSE,               -- Trạng thái xóa mềm Bot
    code VARCHAR(100) UNIQUE NOT NULL,                       -- Mã định danh bot duy nhất
    bot_username VARCHAR(255) NOT NULL,                      -- Username của Bot (ví dụ: MyCloudStorageBot)
    bot_token VARCHAR(255) NOT NULL,                         -- Token API của Telegram Bot (mã hóa ở app level)
    chat_id VARCHAR(255) NOT NULL DEFAULT '',                -- ID kênh/phòng chat Telegram để lưu trữ file
    status VARCHAR(50) NOT NULL,                             -- Trạng thái Bot (ACTIVE, RATE_LIMITED, BANNED)
    total_files_handled BIGINT NOT NULL DEFAULT 0             -- Thống kê số lượng file chunk con Bot đang chứa
);

-- -----------------------------------------------------------------------------
-- 5. BẢNG folders: Quản lý thư mục ảo của người dùng (Cấu trúc cây vô hạn cấp)
-- -----------------------------------------------------------------------------
CREATE TABLE folders (
    id UUID PRIMARY KEY,                                     -- ID duy nhất của thư mục
    created_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, -- Thời gian tạo thư mục
    created_by UUID,                                         -- Người tạo thư mục
    updated_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, -- Thời gian cập nhật gần nhất
    updated_by UUID,                                         -- Người cập nhật gần nhất
    version INT NOT NULL DEFAULT 1,                          -- Phiên bản thư mục
    is_deleted BOOLEAN NOT NULL DEFAULT FALSE,               -- Trạng thái xóa mềm thư mục
    code VARCHAR(100) NOT NULL,                               -- Mã thư mục ảo duy nhất
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE, -- Thư mục thuộc về người dùng nào
    parent_folder_id UUID REFERENCES folders(id) ON DELETE CASCADE, -- ID thư mục cha (NULL = Thư mục gốc - Root)
    folder_name VARCHAR(255) NOT NULL,                       -- Tên thư mục (ví dụ: Tài liệu học tập, Phim ảnh)
    color_code VARCHAR(50),                                  -- Mã màu đại diện cho thư mục trên giao diện UI
    is_starred BOOLEAN NOT NULL DEFAULT FALSE,                -- Đánh dấu sao thư mục quan trọng
    bot_id UUID REFERENCES telegram_bots(id) ON DELETE CASCADE -- Thư mục trong bot nào
);

-- -----------------------------------------------------------------------------
-- 6. BẢNG files: Quản lý metadata tổng quan của file logic hiển thị với người dùng
-- -----------------------------------------------------------------------------
CREATE TABLE files (
    id UUID PRIMARY KEY,                                     -- ID duy nhất của file
    created_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, -- Thời gian tải lên file
    created_by UUID,                                         -- Người tải lên file
    updated_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, -- Thời gian cập nhật gần nhất
    updated_by UUID,                                         -- Người cập nhật gần nhất
    version INT NOT NULL DEFAULT 1,                          -- Phiên bản file
    is_deleted BOOLEAN NOT NULL DEFAULT FALSE,               -- Trạng thái xóa mềm file
    code VARCHAR(100) NOT NULL,                               -- Mã tệp tin duy nhất
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE, -- File thuộc về người dùng nào
    folder_id UUID REFERENCES folders(id) ON DELETE SET NULL, -- Nằm trong thư mục nào (NULL = thư mục gốc)
    file_name VARCHAR(255) NOT NULL,                         -- Tên tệp tin gốc của người dùng (ví dụ: movie.mp4)
    mime_type VARCHAR(100) NOT NULL,                         -- Định dạng MIME của file (ví dụ: video/mp4, image/png)
    size_bytes BIGINT NOT NULL,                              -- Tổng dung lượng tệp tin gốc (byte)
    is_video BOOLEAN NOT NULL DEFAULT FALSE,                 -- Đánh dấu nhanh tệp video phục vụ xử lý luồng stream
    is_starred BOOLEAN NOT NULL DEFAULT FALSE,               -- Đánh dấu sao tệp tin quan trọng
    bot_id UUID REFERENCES telegram_bots(id) ON DELETE CASCADE, -- Tệp trong bot nào
    upload_status VARCHAR(50) NOT NULL                       -- Trạng thái xử lý (PENDING, UPLOADING, PROCESSING, COMPLETED, FAILED)
);

-- -----------------------------------------------------------------------------
-- 7. BẢNG video_metadata: Siêu dữ liệu bổ trợ riêng cho video (Quan hệ 1:1 với files)
-- -----------------------------------------------------------------------------
CREATE TABLE video_metadata (
    id UUID PRIMARY KEY,                                     -- ID duy nhất của bản ghi metadata
    created_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, -- Thời gian khởi tạo metadata
    created_by UUID,                                         -- Người khởi tạo
    updated_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, -- Thời gian cập nhật gần nhất
    updated_by UUID,                                         -- Người cập nhật gần nhất
    version INT NOT NULL DEFAULT 1,                          -- Phiên bản bản ghi
    is_deleted BOOLEAN NOT NULL DEFAULT FALSE,               -- Trạng thái xóa mềm
    code VARCHAR(100) NOT NULL,                               -- Mã siêu dữ liệu video
    file_id UUID UNIQUE NOT NULL REFERENCES files(id) ON DELETE CASCADE, -- Thuộc về tệp tin video nào
    duration_seconds INT NOT NULL,                           -- Thời lượng của video (giây)
    resolution VARCHAR(50),                                  -- Độ phân giải hiển thị video (ví dụ: 1080p, 720p)
    video_codec VARCHAR(50),                                 -- Bộ giải mã hình ảnh video (ví dụ: h264, h265, vp9)
    audio_codec VARCHAR(50),                                 -- Bộ giải mã âm thanh đi kèm video (ví dụ: aac, mp3, opus)
    has_thumbnail BOOLEAN NOT NULL DEFAULT FALSE,            -- Có ảnh xem trước thumbnail hay không
    thumbnail_telegram_file_id VARCHAR(255)                  -- ID ảnh xem trước lưu trên máy chủ Telegram
);

-- -----------------------------------------------------------------------------
-- 8. BẢNG file_chunks: Phân đoạn vật lý của file logic khi chia nhỏ lưu trữ (1:N với files)
-- -----------------------------------------------------------------------------
CREATE TABLE file_chunks (
    id UUID PRIMARY KEY,                                     -- ID duy nhất của mảnh file
    created_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, -- Thời gian tải lên mảnh
    created_by UUID,                                         -- Người tải lên mảnh
    updated_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, -- Thời gian cập nhật gần nhất
    updated_by UUID,                                         -- Người cập nhật gần nhất
    version INT NOT NULL DEFAULT 1,                          -- Phiên bản bản ghi
    is_deleted BOOLEAN NOT NULL DEFAULT FALSE,               -- Trạng thái xóa mềm mảnh
    code VARCHAR(100) NOT NULL,                               -- Mã mảnh phân đoạn tệp tin
    file_id UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE, -- Mảnh này thuộc về file logic nào
    bot_id UUID NOT NULL REFERENCES telegram_bots(id) ON DELETE RESTRICT, -- Do Telegram bot nào trong pool lưu trữ quản lý
    chunk_index INT NOT NULL,                                -- Thứ tự phân đoạn trong file (bắt đầu từ 0)
    start_byte BIGINT NOT NULL,                              -- Byte khởi đầu phân đoạn trong file gốc (ví dụ: 0)
    end_byte BIGINT NOT NULL,                                -- Byte kết thúc phân đoạn trong file gốc (ví dụ: 10485760)
    chunk_size_bytes BIGINT NOT NULL,                        -- Dung lượng thực tế của mảnh phân đoạn (byte)
    telegram_file_id VARCHAR(255) NOT NULL,                  -- File ID do Telegram cấp để phục vụ tải xuống trực tiếp
    telegram_message_id BIGINT NOT NULL                      -- ID tin nhắn chứa tệp tin trên cuộc trò chuyện Telegram Bot
);

-- -----------------------------------------------------------------------------
-- TỐI ƯU HÓA CHỈ MỤC (INDEX)
-- Hỗ trợ truy vấn dải byte (Range search) với tốc độ cực nhanh khi người dùng
-- tua (seek) video trực tuyến trên trình duyệt (Streaming).
-- -----------------------------------------------------------------------------
CREATE INDEX idx_file_chunks_lookup ON file_chunks(file_id, start_byte, end_byte);


-- =============================================================================
-- BỔ SUNG CHÚ THÍCH (COMMENT) CHO CÁC BẢNG VÀ CỘT DỮ LIỆU TRONG METADATA CỦA DB
-- =================================----------------============================

-- 1. Bảng users
COMMENT ON TABLE users IS 'Bảng lưu trữ thông tin tài khoản đăng nhập và trạng thái của người dùng';
COMMENT ON COLUMN users.id IS 'ID duy nhất của tài khoản (UUID)';
COMMENT ON COLUMN users.created_date IS 'Thời gian khởi tạo tài khoản';
COMMENT ON COLUMN users.created_by IS 'ID tài khoản của người tạo bản ghi này';
COMMENT ON COLUMN users.updated_date IS 'Thời gian cập nhật thông tin tài khoản gần nhất';
COMMENT ON COLUMN users.updated_by IS 'ID tài khoản của người cập nhật bản ghi này gần nhất';
COMMENT ON COLUMN users.version IS 'Số phiên bản dùng cho Khóa Lạc Quan (Optimistic Locking) tránh xung đột đồng thời';
COMMENT ON COLUMN users.is_deleted IS 'Trạng thái xóa mềm (Soft Delete)';
COMMENT ON COLUMN users.is_admin IS 'Tài khoản admin được khởi tạo từ đầu';
COMMENT ON COLUMN users.username IS 'Tên đăng nhập duy nhất dùng để truy cập hệ thống';
COMMENT ON COLUMN users.password_hash IS 'Mã băm mật khẩu bảo mật (bcrypt, argon2...)';
COMMENT ON COLUMN users.status IS 'Trạng thái tài khoản (ACTIVE, LOCKED, BANNED)';
COMMENT ON COLUMN users.code IS 'Mã định danh người dùng duy nhất';
COMMENT ON COLUMN users.full_name IS 'Họ và tên đầy đủ của người dùng';
COMMENT ON COLUMN users.email IS 'Địa chỉ email đăng ký duy nhất';
COMMENT ON COLUMN users.phone IS 'Số điện thoại cá nhân liên hệ';
COMMENT ON COLUMN users.department_name IS 'Tên bộ phận/phòng ban làm việc (nếu là nhân viên)';
COMMENT ON COLUMN users.storage_quota_bytes IS 'Định mức dung lượng tối đa được lưu trữ (byte)';
COMMENT ON COLUMN users.used_storage_bytes IS 'Tổng dung lượng tệp tin người dùng đang sử dụng thực tế (byte)';

-- 2. Bảng roles
COMMENT ON TABLE roles IS 'Bảng vai trò người dùng trong hệ thống (RBAC)';
COMMENT ON COLUMN roles.id IS 'ID duy nhất của vai trò (UUID)';
COMMENT ON COLUMN roles.created_date IS 'Thời gian tạo vai trò';
COMMENT ON COLUMN roles.created_by IS 'ID người tạo vai trò';
COMMENT ON COLUMN roles.updated_date IS 'Thời gian cập nhật vai trò gần nhất';
COMMENT ON COLUMN roles.updated_by IS 'ID người cập nhật gần nhất';
COMMENT ON COLUMN roles.version IS 'Số phiên bản dùng cho Khóa Lạc Quan (Optimistic Locking)';
COMMENT ON COLUMN roles.is_deleted IS 'Trạng thái xóa mềm';
COMMENT ON COLUMN roles.code IS 'Mã định danh vai trò duy nhất dùng trong code (ví dụ: ROLE_ADMIN)';
COMMENT ON COLUMN roles.name IS 'Tên hiển thị trực quan của vai trò';
COMMENT ON COLUMN roles.description IS 'Mô tả chi tiết quyền hạn và trách nhiệm của vai trò';
COMMENT ON COLUMN roles.permissions IS 'Danh sách các quyền hạn được lưu dưới dạng JSON String';


-- 3. Bảng user_roles
COMMENT ON TABLE user_roles IS 'Bảng liên kết nhiều-nhiều giữa tài khoản và vai trò';
COMMENT ON COLUMN user_roles.id IS 'ID duy nhất của bản ghi liên kết';
COMMENT ON COLUMN user_roles.created_date IS 'Thời gian thiết lập liên kết';
COMMENT ON COLUMN user_roles.created_by IS 'ID người thiết lập';
COMMENT ON COLUMN user_roles.updated_date IS 'Thời gian cập nhật liên kết gần nhất';
COMMENT ON COLUMN user_roles.updated_by IS 'ID người cập nhật gần nhất';
COMMENT ON COLUMN user_roles.version IS 'Số phiên bản bản ghi';
COMMENT ON COLUMN user_roles.is_deleted IS 'Trạng thái xóa mềm';
COMMENT ON COLUMN user_roles.user_id IS 'ID tài khoản người dùng';
COMMENT ON COLUMN user_roles.role_id IS 'ID vai trò được gán';

-- 4. Bảng telegram_bots
COMMENT ON TABLE telegram_bots IS 'Bảng quản lý danh sách các Telegram Bot đóng vai trò làm node lưu trữ vật lý';
COMMENT ON COLUMN telegram_bots.id IS 'ID duy nhất của bot (UUID)';
COMMENT ON COLUMN telegram_bots.created_date IS 'Thời gian tích hợp Bot vào hệ thống';
COMMENT ON COLUMN telegram_bots.created_by IS 'ID người cấu hình Bot';
COMMENT ON COLUMN telegram_bots.updated_date IS 'Thời gian cập nhật trạng thái gần nhất';
COMMENT ON COLUMN telegram_bots.updated_by IS 'ID người cập nhật gần nhất';
COMMENT ON COLUMN telegram_bots.version IS 'Số phiên bản bản ghi';
COMMENT ON COLUMN telegram_bots.is_deleted IS 'Trạng thái xóa mềm Bot';
COMMENT ON COLUMN telegram_bots.code IS 'Mã định danh Bot nội bộ duy nhất';
COMMENT ON COLUMN telegram_bots.bot_username IS 'Username định danh của Bot trên Telegram (ví dụ: MyDiskBot)';
COMMENT ON COLUMN telegram_bots.bot_token IS 'Mã Token kết nối API Telegram Bot';
COMMENT ON COLUMN telegram_bots.chat_id IS 'ID cuộc trò chuyện/kênh Telegram Bot dùng để upload và lưu trữ file';
COMMENT ON COLUMN telegram_bots.status IS 'Trạng thái kết nối của Bot (ACTIVE, RATE_LIMITED, BANNED)';
COMMENT ON COLUMN telegram_bots.total_files_handled IS 'Tổng số lượng file chunk vật lý Bot này đang lưu trữ thực tế';

-- 5. Bảng folders
COMMENT ON TABLE folders IS 'Bảng quản lý cây thư mục ảo vô hạn cấp của người dùng';
COMMENT ON COLUMN folders.id IS 'ID duy nhất của thư mục (UUID)';
COMMENT ON COLUMN folders.created_date IS 'Thời gian tạo thư mục ảo';
COMMENT ON COLUMN folders.created_by IS 'ID khách hàng tạo';
COMMENT ON COLUMN folders.updated_date IS 'Thời gian cập nhật gần nhất';
COMMENT ON COLUMN folders.updated_by IS 'ID người cập nhật gần nhất';
COMMENT ON COLUMN folders.version IS 'Số phiên bản thư mục';
COMMENT ON COLUMN folders.is_deleted IS 'Trạng thái xóa mềm thư mục ảo';
COMMENT ON COLUMN folders.code IS 'Mã thư mục ảo duy nhất';
COMMENT ON COLUMN folders.user_id IS 'ID tài khoản người dùng sở hữu thư mục ảo trỏ tới users.id';
COMMENT ON COLUMN folders.bot_id IS 'ID bot dùng làm nơi lưu trữ vật lý cho thư mục ảo này trỏ tới telegram_bots.id';
COMMENT ON COLUMN folders.parent_folder_id IS 'ID thư mục cha chứa thư mục này (NULL nếu nằm ở thư mục gốc)';
COMMENT ON COLUMN folders.folder_name IS 'Tên hiển thị trực quan của thư mục ảo';
COMMENT ON COLUMN folders.color_code IS 'Mã màu tùy chỉnh đại diện thư mục ảo hiển thị trên UI';

-- 6. Bảng files
COMMENT ON TABLE files IS 'Bảng quản lý siêu dữ liệu (metadata) của file ảo hiển thị với người dùng';
COMMENT ON COLUMN files.id IS 'ID duy nhất của tệp tin logic (UUID)';
COMMENT ON COLUMN files.created_date IS 'Thời gian tải lên tệp tin';
COMMENT ON COLUMN files.created_by IS 'ID khách hàng tải tệp';
COMMENT ON COLUMN files.updated_date IS 'Thời gian cập nhật tệp gần nhất';
COMMENT ON COLUMN files.updated_by IS 'ID người cập nhật gần nhất';
COMMENT ON COLUMN files.version IS 'Số phiên bản bản ghi';
COMMENT ON COLUMN files.is_deleted IS 'Trạng thái xóa mềm tệp tin';
COMMENT ON COLUMN files.code IS 'Mã định danh tệp tin logic duy nhất';
COMMENT ON COLUMN files.user_id IS 'ID tài khoản người dùng sở hữu tệp trỏ tới users.id';
COMMENT ON COLUMN files.folder_id IS 'ID thư mục ảo cha chứa tệp (NULL nếu nằm ở thư mục gốc)';
COMMENT ON COLUMN files.bot_id IS 'ID bot dùng làm nơi lưu trữ vật lý';
COMMENT ON COLUMN files.file_name IS 'Tên nguyên bản của tệp tin khi tải lên (ví dụ: bbc_document.pdf)';
COMMENT ON COLUMN files.mime_type IS 'Định dạng tài nguyên MIME của tệp (ví dụ: video/mp4, image/jpeg, application/pdf)';
COMMENT ON COLUMN files.size_bytes IS 'Tổng dung lượng nguyên bản của tệp tin gốc (byte)';
COMMENT ON COLUMN files.is_video IS 'Đánh dấu nhanh tệp video hỗ trợ bộ phát trực tiếp phân tuyến dữ liệu';
COMMENT ON COLUMN files.upload_status IS 'Tiến trình tải lên và xử lý tệp (PENDING, UPLOADING, PROCESSING, COMPLETED, FAILED)';

-- 7. Bảng video_metadata
COMMENT ON TABLE video_metadata IS 'Bảng lưu trữ thông số kỹ thuật tối ưu hóa phát và nén riêng cho Video';
COMMENT ON COLUMN video_metadata.id IS 'ID duy nhất của siêu dữ liệu video (UUID)';
COMMENT ON COLUMN video_metadata.created_date IS 'Thời gian khởi tạo thông tin';
COMMENT ON COLUMN video_metadata.created_by IS 'ID người tạo thông tin';
COMMENT ON COLUMN video_metadata.updated_date IS 'Thời gian cập nhật gần nhất';
COMMENT ON COLUMN video_metadata.updated_by IS 'ID người cập nhật gần nhất';
COMMENT ON COLUMN video_metadata.version IS 'Số phiên bản bản ghi';
COMMENT ON COLUMN video_metadata.is_deleted IS 'Trạng thái xóa mềm';
COMMENT ON COLUMN video_metadata.code IS 'Mã định danh siêu dữ liệu video';
COMMENT ON COLUMN video_metadata.file_id IS 'ID file liên kết 1:1 trỏ tới files.id';
COMMENT ON COLUMN video_metadata.duration_seconds IS 'Tổng thời lượng phát của video (tính bằng giây)';
COMMENT ON COLUMN video_metadata.resolution IS 'Độ phân giải hiển thị video (ví dụ: 1080p, 2K, 720p)';
COMMENT ON COLUMN video_metadata.video_codec IS 'Bộ codec giải mã hình ảnh video (ví dụ: h264, h265, vp9)';
COMMENT ON COLUMN video_metadata.audio_codec IS 'Bộ codec giải mã âm thanh đi kèm video (ví dụ: aac, mp3, opus)';
COMMENT ON COLUMN video_metadata.has_thumbnail IS 'Có ảnh xem trước preview của video hay không';
COMMENT ON COLUMN video_metadata.thumbnail_telegram_file_id IS 'ID tệp ảnh preview của video được lưu trên máy chủ Telegram';

-- 8. Bảng file_chunks
COMMENT ON TABLE file_chunks IS 'Bảng quản lý thông tin dải byte vật lý của phân đoạn file được lưu trên Telegram';
COMMENT ON COLUMN file_chunks.id IS 'ID duy nhất của phân đoạn file (UUID)';
COMMENT ON COLUMN file_chunks.created_date IS 'Thời gian tải phân đoạn lên Telegram';
COMMENT ON COLUMN file_chunks.created_by IS 'ID người thực hiện tải lên';
COMMENT ON COLUMN file_chunks.updated_date IS 'Thời gian cập nhật gần nhất';
COMMENT ON COLUMN file_chunks.updated_by IS 'ID người cập nhật gần nhất';
COMMENT ON COLUMN file_chunks.version IS 'Số phiên bản bản ghi';
COMMENT ON COLUMN file_chunks.is_deleted IS 'Trạng thái xóa mềm phân đoạn';
COMMENT ON COLUMN file_chunks.code IS 'Mã định danh phân đoạn tệp tin';
COMMENT ON COLUMN file_chunks.file_id IS 'ID file cha sở hữu phân đoạn trỏ tới files.id';
COMMENT ON COLUMN file_chunks.bot_id IS 'ID Telegram bot quản lý lưu trữ phân đoạn trỏ tới telegram_bots.id';
COMMENT ON COLUMN file_chunks.chunk_index IS 'Thứ tự chỉ số của phân đoạn trong file (bắt đầu từ 0)';
COMMENT ON COLUMN file_chunks.start_byte IS 'Byte khởi đầu phân đoạn trong file gốc (ví dụ: 0)';
COMMENT ON COLUMN file_chunks.end_byte IS 'Byte kết thúc phân đoạn trong file gốc (ví dụ: 10485760)';
COMMENT ON COLUMN file_chunks.chunk_size_bytes IS 'Dung lượng thực tế của mảnh phân đoạn (byte)';
COMMENT ON COLUMN file_chunks.telegram_file_id IS 'Mã định danh file ID do Telegram API cấp để tải tệp tin về';
COMMENT ON COLUMN file_chunks.telegram_message_id IS 'ID tin nhắn chứa file chunk trên cuộc trò chuyện bot Telegram';
