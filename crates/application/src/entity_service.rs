//! Entity Service - 实体 / 关系 / 角色子数据的业务逻辑层。
//!
//! 读操作（list/get/character 子数据）通过 EntityRepositoryPort 完成。
//! 写操作（create/update/delete 实体）统一经由 MutationCommitter 提交，
//! 不再直接调用 repo 做 Canon mutation（提案 四 / 二十四）。
//!
//! 关系与事实的语义化写入（EndRelation / SupersedeFact 等）在后续阶段接入。

use anyhow::Result;
use domain::mutation::MutationCommand;
use domain::ports::{EntityRepositoryPort, ProjectResolverPort};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

use crate::mutation::MutationCommitter;

/// Entity Service - 实体服务
pub struct EntityService {
    repo: Arc<dyn EntityRepositoryPort>,
    committer: Arc<MutationCommitter>,
    resolver: Arc<dyn ProjectResolverPort>,
    /// 见 `new()` 的说明
    actor: &'static str,
}

impl EntityService {
    /// `actor` 记录「这次改动是谁发起的」（`MutationSource::as_str()`：user / ai / system），
    /// 会写进档案历史快照——用户要能区分「我自己改的」和「AI 改的」。
    /// 同一个 service 实例的来源是固定的：HTTP 接口构造的是 user，AI 工具构造的是 ai。
    pub fn new(
        repo: Arc<dyn EntityRepositoryPort>,
        committer: Arc<MutationCommitter>,
        resolver: Arc<dyn ProjectResolverPort>,
        actor: &'static str,
    ) -> Self {
        Self {
            repo,
            committer,
            resolver,
            actor,
        }
    }

    pub async fn list_entities(
        &self,
        world_id: Uuid,
        entity_type: Option<&str>,
    ) -> Result<Vec<Value>> {
        self.repo.list_entities(world_id, entity_type).await
    }

    /// 有界列表（目录页）：返回本页实体 + 总数。
    ///
    /// 为什么不直接用 `list_entities`：实测它一次返回 33 个实体的全字段 = 36,555 字符
    /// （`description` 占 51%），这些字符进入上下文后每轮请求都要重发。
    /// 调用方给出 limit/offset，并拿到 total——先知道"有多少"，再决定要不要翻页。
    pub async fn list_entities_page(
        &self,
        world_id: Uuid,
        entity_type: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<Value>, usize)> {
        let all = self.repo.list_entities(world_id, entity_type).await?;
        let total = all.len();
        let items = all.into_iter().skip(offset).take(limit).collect();
        Ok((items, total))
    }

    pub async fn get_entity(&self, id: Uuid) -> Result<Option<Value>> {
        self.repo.get_entity(id).await
    }

    /// 创建实体：经由 MutationCommitter（提案 四）。
    pub async fn create_entity(
        &self,
        world_id: Uuid,
        entity_type_name: &str,
        name: &str,
        summary: Option<&str>,
        description: Option<&str>,
    ) -> Result<Value> {
        let project_id = self
            .resolver
            .project_id_for_world(world_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("project not found for world {}", world_id))?;

        let cmd = MutationCommand::create_entity(
            project_id,
            world_id,
            entity_type_name,
            name,
            summary,
            description,
        );
        // 实体创建属于「影响该世界的 Canon 写」：显式声明 affected_worlds，
        // 让提交者在同一事务内推进这个世界的 world_version（ChatGPT 评审 P2/B）。
        let results = self
            .committer
            .commit_with_worlds(cmd, vec![world_id])
            .await?;
        let result = results
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("mutation returned no result"))?;
        let entity_id = *result
            .created_ids
            .first()
            .ok_or_else(|| anyhow::anyhow!("mutation did not return created entity"))?;
        self.repo
            .get_entity(entity_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("created entity {} not found", entity_id))
    }

    /// 更新实体：经由 MutationCommitter（提案 四）。带乐观锁。
    pub async fn update_entity(
        &self,
        id: Uuid,
        name: Option<&str>,
        summary: Option<&str>,
        description: Option<&str>,
        attributes: Option<&Value>,
    ) -> Result<Value> {
        let existing = self
            .repo
            .get_entity(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("entity {} not found", id))?;
        let project_id = extract_project_id(&existing)?;
        let expected_version = extract_version(&existing)?;

        let cmd = MutationCommand::update_entity(
            project_id,
            id,
            Some(expected_version),
            name.map(|s| s.to_string()),
            summary.map(|s| s.to_string()),
            description.map(|s| s.to_string()),
            attributes.map(|v| v.clone()),
        );
        self.committer.commit(cmd).await?;
        self.repo
            .get_entity(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("entity {} disappeared after update", id))
    }

    /// 删除实体：经由 MutationCommitter（语义化软删除，绝不物理 DELETE）。
    pub async fn delete_entity(&self, id: Uuid) -> Result<Value> {
        let existing = self
            .repo
            .get_entity(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("entity {} not found", id))?;
        let project_id = extract_project_id(&existing)?;
        let expected_version = extract_version(&existing)?;

        let cmd = MutationCommand::delete_entity(project_id, id, expected_version);
        self.committer.commit(cmd).await?;
        Ok(existing)
    }

    pub async fn list_relations(&self, world_id: Uuid) -> Result<Vec<Value>> {
        self.repo.list_relations(world_id).await
    }

    /// 有界列表（目录页）：返回本页 + 总数。（关系边）
    ///
    /// 过渡实现：repository 还没下推 LIMIT/OFFSET，先取回再切片——
    /// 目的是**把返回给模型的体积有界化**（实测无界列表一次能到 42 万字符）；
    /// 等下推实现后就替换成真正的分页查询。
    pub async fn list_relations_page(
        &self,
        world_id: Uuid,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<serde_json::Value>, usize)> {
        let all = self.list_relations(world_id).await?;
        let total = all.len();
        let items = all.into_iter().skip(offset).take(limit).collect();
        Ok((items, total))
    }


    pub async fn create_relation(
        &self,
        source_entity_id: Uuid,
        target_entity_id: Uuid,
        relation_type: &str,
        description: Option<&str>,
    ) -> Result<Value> {
        self.repo
            .create_relation(source_entity_id, target_entity_id, relation_type, description)
            .await
    }

    /// 修改关系（关系类型 / 描述），语义化写入并留痕（ReviseRelation）。
    ///
    /// 为什么需要它：关系原先只有 create / end，描述写错就只能"结束旧边 + 重建新边"，
    /// 结果是 id 变了、时间线断成两段。改属性不该付这个代价。
    /// `None` 表示该项不改；两项都没给直接报错（避免一次什么也没改的空写）。
    pub async fn revise_relation(
        &self,
        id: Uuid,
        relation_type: Option<&str>,
        description: Option<&str>,
    ) -> Result<()> {
        if relation_type.is_none() && description.is_none() {
            anyhow::bail!("revise_relation 至少要给 relation_type 或 description 之一");
        }
        let project_id = self
            .resolver
            .project_id_for_relation(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("project not found for relation {}", id))?;
        let cmd = MutationCommand::revise_relation(project_id, id, relation_type, description);
        self.committer.commit(cmd).await?;
        Ok(())
    }

    /// 删除关系：语义化结束（EndRelation），绝不物理 DELETE（提案 五）。
    pub async fn delete_relation(&self, id: Uuid) -> Result<()> {
        let project_id = self
            .resolver
            .project_id_for_relation(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("project not found for relation {}", id))?;
        let cmd = MutationCommand::end_relation(project_id, id, None);
        self.committer.commit(cmd).await?;
        Ok(())
    }

    pub async fn get_character_profile(&self, id: Uuid) -> Result<Option<Value>> {
        let Some(profile) = self.repo.get_character_profile(id).await? else {
            return Ok(None);
        };
        Ok(Some(self.with_entity_name(id, profile).await?))
    }

    pub async fn get_character_state(&self, id: Uuid) -> Result<Option<Value>> {
        self.repo.get_character_state(id).await
    }

    /// 把实体的权威名称补进档案返回值（原地覆盖同名键）。
    ///
    /// 为什么必须有这一步：实体名只存在 `entity.name` 一处（关系的两端、列表、
    /// 前端卡片用的都是它）；而 `character_profile.name` 是历史遗留的冗余列，
    /// **它常常是空的**（实测 12 个角色里 8 个为 NULL）。两处并存的结果是
    /// 「档案读出来 name=null、写入又只回显传进去的字段」，调用方以为名字丢了。
    ///
    /// 这里以 `entity.name` 为准覆盖，让档案返回的名字恒等于实体真名——
    /// 单一真源，不给调用方留第二个可能为空的字段去纠结。
    async fn with_entity_name(&self, id: Uuid, mut profile: Value) -> Result<Value> {
        let entity = self
            .repo
            .get_entity(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("entity {} not found", id))?;
        let entity_name = entity.get("name").cloned().unwrap_or(Value::Null);

        match profile.as_object_mut() {
            Some(obj) => {
                obj.insert("name".to_string(), entity_name);
            }
            None => {
                anyhow::bail!(
                    "{} 的档案返回值不是 JSON 对象，无法补入实体名",
                    id
                );
            }
        }
        Ok(profile)
    }

    /// 改档案前先确认实体存在。
    ///
    /// 否则会走到 UPSERT，由 `entity_profile` 的外键失败兜底报出来——
    /// 对外是 500（服务器错误），而真实原因是「这个实体不存在」，
    /// 应该是 404。这里的错误文案与 `update_entity` 保持一致的 "entity {} not found",
    /// 便于 handler 统一映射成 404。
    async fn ensure_entity_exists(&self, id: Uuid) -> Result<()> {
        self.repo
            .get_entity(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("entity {} not found", id))?;
        Ok(())
    }

    pub async fn update_character_profile(&self, id: Uuid, profile: Value) -> Result<Value> {
        self.ensure_entity_exists(id).await?;
        // 改动前留档（拿不到旧档案说明是首次创建，没有「改前」可留）
        if let Some(before) = self.repo.get_character_profile(id).await? {
            self.repo.snapshot_profile(id, "character", &before, self.actor).await?;
        }
        let written = self.repo.update_character_profile(id, profile, self.actor).await?;
        // 写入路径同样补实体名：否则「没传 name 就不回显 name」会让调用方
        // 误以为名字丢了（实测 AI 就是这样误判的）
        self.with_entity_name(id, written).await
    }

    pub async fn update_character_state(&self, id: Uuid, state: Value) -> Result<Value> {
        self.ensure_entity_exists(id).await?;
        if let Some(before) = self.repo.get_character_state(id).await? {
            self.repo.snapshot_profile(id, "character", &before, self.actor).await?;
        }
        self.repo.update_character_state(id, state, self.actor).await
    }

    pub async fn get_location_profile(&self, id: Uuid) -> Result<Option<Value>> {
        let Some(profile) = self.repo.get_location_profile(id).await? else {
            return Ok(None);
        };
        Ok(Some(self.with_entity_name(id, profile).await?))
    }

    pub async fn upsert_location_profile(&self, id: Uuid, profile: Value) -> Result<Value> {
        self.ensure_entity_exists(id).await?;
        if let Some(before) = self.repo.get_location_profile(id).await? {
            self.repo.snapshot_profile(id, "location", &before, self.actor).await?;
        }
        let written = self.repo.upsert_location_profile(id, profile, self.actor).await?;
        self.with_entity_name(id, written).await
    }

    pub async fn get_faction_profile(&self, id: Uuid) -> Result<Option<Value>> {
        let Some(profile) = self.repo.get_faction_profile(id).await? else {
            return Ok(None);
        };
        Ok(Some(self.with_entity_name(id, profile).await?))
    }

    pub async fn upsert_faction_profile(&self, id: Uuid, profile: Value) -> Result<Value> {
        self.ensure_entity_exists(id).await?;
        if let Some(before) = self.repo.get_faction_profile(id).await? {
            self.repo.snapshot_profile(id, "faction", &before, self.actor).await?;
        }
        let written = self.repo.upsert_faction_profile(id, profile, self.actor).await?;
        self.with_entity_name(id, written).await
    }

    pub async fn get_character_knowledge(&self, id: Uuid) -> Result<Vec<Value>> {
        self.repo.get_character_knowledge(id).await
    }

    pub async fn get_character_relationships(&self, id: Uuid) -> Result<Vec<Value>> {
        self.repo.get_character_relationships(id).await
    }
}

fn extract_project_id(entity: &Value) -> Result<Uuid> {
    entity
        .get("project_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| anyhow::anyhow!("entity missing project_id"))
}

fn extract_version(entity: &Value) -> Result<i32> {
    entity
        .get("version")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .ok_or_else(|| anyhow::anyhow!("entity missing version"))
}
