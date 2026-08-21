//! Application Services - 应用服务层
//!
//! 负责 World/Entity/Narrative/Generation 的业务逻辑。
//! 依赖 domain + infrastructure (db)，不依赖 runtime。

pub mod command;
pub mod mutation;

pub mod world_service;
pub mod narrative_service;
pub mod generation_service;
pub mod generation_executor;
pub mod extraction_executor;
pub mod timeline_service;
pub mod storyline_service;
pub mod foreshadow_service;
pub mod approval_service;
pub mod proposal_service;
pub mod project_service;
pub mod rule_service;
pub mod history_service;
pub mod snapshot_service;
pub mod trace_service;
pub mod entity_service;
