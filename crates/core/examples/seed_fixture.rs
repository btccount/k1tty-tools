use k1tty_core::{
    converter,
    service::{AppPaths, Service, PROVIDER_ORDER},
};
use serde_json::json;
use std::{path::PathBuf, sync::Arc};
fn main() -> anyhow::Result<()> {
    let root = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or_else(|| anyhow::anyhow!("Explicit sandbox root required"))?,
    );
    let service = Service::new(AppPaths::isolated(&root), Arc::new(|_| {}))?;
    let doc = json!({"meta":{"title":"Проверка переноса · K1tty Tools","directory":root.join("project"),"source_provider":"fixture","source_id":"fixture"},"entries":[
        {"kind":"system","subtype":"system","label":"Система","text":"Тестовый диалог. Сохраняй структуру файлов и Unicode.","ts":"2026-11-20T01:00:00Z"},
        {"kind":"user","subtype":"user","label":"Вы","text":"Перенеси историю с сохранением контекста. Пример: привет, світ 🌍","ts":"2026-11-20T01:00:01Z"},
        {"kind":"tool","subtype":"thinking","tool":"thinking","label":"Размышления","text":"Сначала проверю форматы данных, затем резервные копии.","ts":"2026-11-20T01:00:02Z"},
        {"kind":"assistant","subtype":"text","label":"Assistant","text":"## План\n\n1. Сохранить сообщения и метаданные.\n2. Проверить архив `.k1tty`.\n\n```rust\nfn main() { println!(\"Привет!\"); }\n```","ts":"2026-11-20T01:00:03Z"},
        {"kind":"tool","subtype":"tool_use","tool":"read_file","label":"read_file","text":"Cargo.toml: workspace with core and desktop crates","status":"ok","ts":"2026-11-20T01:00:04Z"},
        {"kind":"user","subtype":"user","label":"Вы","text":"Хорошо, продолжай. Сохрани иконки и шрифты 1 в 1.","ts":"2026-11-20T01:00:05Z"},
        {"kind":"assistant","subtype":"text","label":"Assistant","text":"Готово. **Интерфейс сохранён**, а обработка истории выполняется на Rust.","ts":"2026-11-20T01:00:06Z"}
    ]});
    for id in PROVIDER_ORDER {
        let p = service.provider(id)?;
        let result = converter::import_doc_to_provider(&doc, p.as_ref(), None, true)?;
        println!("{result}");
    }
    Ok(())
}
