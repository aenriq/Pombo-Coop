mod color_system;
mod mock_data;
mod shell;
mod ui_state;

use gpui::*;
use std::borrow::Cow;

use shell::{bind_keys, AppShell};

struct EmbeddedAssets;

impl AssetSource for EmbeddedAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let asset = match path {
            "icons/lucide-square-minus.svg" => {
                Some(include_bytes!("../assets/icons/lucide-square-minus.svg").as_slice())
            }
            "icons/lucide-square-plus.svg" => {
                Some(include_bytes!("../assets/icons/lucide-square-plus.svg").as_slice())
            }
            "icons/lucide-square-dot.svg" => {
                Some(include_bytes!("../assets/icons/lucide-square-dot.svg").as_slice())
            }
            _ => None,
        };
        Ok(asset.map(Cow::Borrowed))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        if path == "icons" {
            Ok(vec![
                "icons/lucide-square-minus.svg".into(),
                "icons/lucide-square-plus.svg".into(),
                "icons/lucide-square-dot.svg".into(),
            ])
        } else {
            Ok(vec![])
        }
    }
}

fn main() {
    gpui_platform::application()
        .with_assets(EmbeddedAssets)
        .run(|cx: &mut App| {
            bind_keys(cx);

            cx.open_window(WindowOptions::default(), |window, cx| {
                let shell = cx.new(AppShell::new);
                let root_focus_handle = shell.read(cx).root_focus_handle();
                window.focus(&root_focus_handle, cx);
                shell
            })
            .expect("failed to open Agent Manager UI window");
        });
}
