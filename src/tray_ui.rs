use crate::{
    battery::Status,
    icons::Renderer,
    platform,
    settings::{self, DisplayMode},
};
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

pub(crate) enum Action {
    None,
    Refresh,
    Quit,
}

pub(crate) struct TrayUi {
    tray: TrayIcon,
    renderer: Renderer,
    status_item: MenuItem,
    percentage_item: CheckMenuItem,
    battery_item: CheckMenuItem,
    refresh_item: MenuItem,
    quit_item: MenuItem,
    status: Status,
    mode: DisplayMode,
}

impl TrayUi {
    pub(crate) fn new(mode: DisplayMode) -> Result<Self, String> {
        let renderer = Renderer::new()?;
        let status_item = MenuItem::new("Checking battery…", false, None);
        let percentage_item = CheckMenuItem::new(
            "Percentage",
            mode != DisplayMode::Percentage,
            mode == DisplayMode::Percentage,
            None,
        );
        let battery_item = CheckMenuItem::new(
            "Colored battery",
            mode != DisplayMode::Battery,
            mode == DisplayMode::Battery,
            None,
        );
        let refresh_item = MenuItem::new("Refresh", true, None);
        let quit_item = MenuItem::new("Quit", true, None);
        let menu = Menu::new();
        menu.append_items(&[
            &status_item,
            &PredefinedMenuItem::separator(),
            &percentage_item,
            &battery_item,
            &PredefinedMenuItem::separator(),
            &refresh_item,
            &quit_item,
        ])
        .map_err(|error| error.to_string())?;

        let pixels = renderer.render(Status::Unavailable, mode, platform::dark_taskbar());
        let icon = Icon::from_rgba(pixels, 32, 32).map_err(|error| error.to_string())?;
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("MCHOSE K7 V2 Ultra+ — Battery unavailable")
            .with_icon(icon)
            .build()
            .map_err(|error| error.to_string())?;

        Ok(Self {
            tray,
            renderer,
            status_item,
            percentage_item,
            battery_item,
            refresh_item,
            quit_item,
            status: Status::Unavailable,
            mode,
        })
    }

    pub(crate) fn paint(&mut self, status: Status) -> Result<(), String> {
        self.status = status;
        self.repaint()
    }

    pub(crate) fn repaint(&mut self) -> Result<(), String> {
        let label = self.status.label();
        self.status_item.set_text(&label);
        self.tray
            .set_tooltip(Some(format!("MCHOSE K7 V2 Ultra+ — {label}")))
            .map_err(|error| error.to_string())?;
        let pixels = self
            .renderer
            .render(self.status, self.mode, platform::dark_taskbar());
        let icon = Icon::from_rgba(pixels, 32, 32).map_err(|error| error.to_string())?;
        self.tray
            .set_icon(Some(icon))
            .map_err(|error| error.to_string())
    }

    pub(crate) fn apply_reading(&mut self, reading: Result<Status, String>) -> Result<(), String> {
        match reading {
            Ok(status) => self.paint(status),
            Err(error) => {
                crate::logging::write(crate::logging::Level::Warn, &error);
                self.paint(Status::Unavailable)
            }
        }
    }

    pub(crate) fn handle_menu(&mut self, event: MenuEvent) -> Result<Action, String> {
        if event.id == *self.refresh_item.id() {
            return Ok(Action::Refresh);
        }
        if event.id == *self.quit_item.id() {
            return Ok(Action::Quit);
        }
        if event.id == *self.percentage_item.id() {
            self.select_mode(DisplayMode::Percentage)?;
        } else if event.id == *self.battery_item.id() {
            self.select_mode(DisplayMode::Battery)?;
        }
        Ok(Action::None)
    }

    fn select_mode(&mut self, mode: DisplayMode) -> Result<(), String> {
        if mode == self.mode {
            return Ok(());
        }
        self.mode = mode;
        self.percentage_item
            .set_checked(mode == DisplayMode::Percentage);
        self.percentage_item
            .set_enabled(mode != DisplayMode::Percentage);
        self.battery_item.set_checked(mode == DisplayMode::Battery);
        self.battery_item.set_enabled(mode != DisplayMode::Battery);
        settings::save(mode).map_err(|error| format!("Could not save display mode: {error}"))?;
        self.repaint()
    }
}
