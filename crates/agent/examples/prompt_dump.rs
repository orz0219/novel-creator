//! 把 Agent 实际会收到的 system prompt 打出来（调试用）。
//!
//! 存在的理由：system prompt 是**运行时动态拼接**的（基座 + 流程全貌 + 当前阶段 +
//! 本步目标 + 就绪条件 + 项目记忆 + 工具清单 + 协议），而设置页里那个
//! 「预览最终提示词」只是前端自拼的近似值，**不含**中间这几段。
//! 想确认"AI 到底被告知了什么"，只能把真实的拼装结果导出来看。
//!
//! 用法：
//! ```text
//! cargo run -p agent --example prompt_dump -- beats.chapters > /tmp/prompt.txt
//! cargo run -p agent --example prompt_dump -- writing
//! ```
//! 不带参数时用 `beats.chapters`。
//!
//! 注意：这里传空的工具清单与空记忆 —— 本工具只用于查看「流程 / 阶段」相关段落；
//! 真实运行时还要加上 70 个工具的 name/description/字段与项目记忆。

use agent::guide::{find_step, STEPS};
use agent::prompt::{build_system_prompt, DEFAULT_SYSTEM_PROMPT_BASE};

fn main() {
    let step = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "beats.chapters".to_string());

    if find_step(&step).is_none() {
        eprintln!("注意：`{}` 不在流程定义里（共 {} 步），下面展示的是\"阶段 key 无效\"时的提示词。", step, STEPS.len());
    }

    let prompt = build_system_prompt(DEFAULT_SYSTEM_PROMPT_BASE, &step, &[], &[]);
    print!("{}", prompt);
}
