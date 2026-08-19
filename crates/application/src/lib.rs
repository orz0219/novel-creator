//! Application Services - 应用服务层
//!
//! 负责 World/Entity/Narrative/Generation 的业务逻辑。
//! 依赖 domain + infrastructure (db)，不依赖 runtime。

pub mod world_service;
pub mod narrative_service;
pub mod generation_service;
pub mod timeline_service;
pub mod storyline_service;
pub mod approval_service;
pub mod proposal_service;
