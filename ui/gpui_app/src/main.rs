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
            "icons/lucide-chevron-down.svg" => {
                Some(include_bytes!("../assets/icons/lucide-chevron-down.svg").as_slice())
            }
            "icons/lucide-chevron-right.svg" => {
                Some(include_bytes!("../assets/icons/lucide-chevron-right.svg").as_slice())
            }
            "icons/lucide-archive.svg" => {
                Some(include_bytes!("../assets/icons/lucide-archive.svg").as_slice())
            }
            "icons/lucide-folder.svg" => {
                Some(include_bytes!("../assets/icons/lucide-folder.svg").as_slice())
            }
            "icons/lucide-folder-open.svg" => {
                Some(include_bytes!("../assets/icons/lucide-folder-open.svg").as_slice())
            }
            "icons/lucide-folder-plus.svg" => {
                Some(include_bytes!("../assets/icons/lucide-folder-plus.svg").as_slice())
            }
            "icons/lucide-git-branch.svg" => {
                Some(include_bytes!("../assets/icons/lucide-git-branch.svg").as_slice())
            }
            "icons/lucide-settings.svg" => {
                Some(include_bytes!("../assets/icons/lucide-settings.svg").as_slice())
            }
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
                "icons/lucide-chevron-down.svg".into(),
                "icons/lucide-chevron-right.svg".into(),
                "icons/lucide-archive.svg".into(),
                "icons/lucide-folder.svg".into(),
                "icons/lucide-folder-open.svg".into(),
                "icons/lucide-folder-plus.svg".into(),
                "icons/lucide-git-branch.svg".into(),
                "icons/lucide-settings.svg".into(),
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

            let window_options = WindowOptions {
                titlebar: Some(TitlebarOptions {
                    title: Some("AgentManager".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(12.0), px(12.0))),
                }),
                ..WindowOptions::default()
            };

            cx.open_window(window_options, |window, cx| {
                let shell = cx.new(AppShell::new);
                let root_focus_handle = shell.read(cx).root_focus_handle();
                window.focus(&root_focus_handle, cx);
                shell
            })
            .expect("failed to open Agent Manager UI window");
        });
}
