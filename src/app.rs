use crate::{
    battery::Status,
    logging::{self, Level},
    platform::{self, MessageKind},
    settings::DisplayMode,
    tray_ui::{Action, TrayUi},
    worker::Worker,
};
use std::sync::mpsc;
use tray_icon::menu::MenuEvent;

pub(crate) enum Event {
    Reading(Result<Status, String>),
    Menu(MenuEvent),
}

enum Command {
    Run(DisplayMode),
    Quit,
}

pub fn run() -> Result<(), String> {
    match parse_command(std::env::args().skip(1))? {
        Command::Quit => quit_running_instance(),
        Command::Run(mode) => run_single_instance(mode),
    }
}

pub(crate) fn report_error(error: &str) {
    logging::write(Level::Error, error);
    platform::show_message("MCHOSE Tray", error, MessageKind::Error);
}

fn parse_command(args: impl IntoIterator<Item = String>) -> Result<Command, String> {
    let mut mode = crate::settings::load();
    for argument in args {
        match argument.as_str() {
            "--quit" => return Ok(Command::Quit),
            "--percent" => mode = DisplayMode::Percentage,
            "--battery" => mode = DisplayMode::Battery,
            unknown => return Err(format!("Unknown option: {unknown}")),
        }
    }
    Ok(Command::Run(mode))
}

fn quit_running_instance() -> Result<(), String> {
    if platform::request_quit() {
        Ok(())
    } else {
        Err("MCHOSE Tray is not running.".into())
    }
}

fn run_single_instance(mode: DisplayMode) -> Result<(), String> {
    let instance = platform::SingleInstance::acquire()?;
    if instance.already_exists() {
        platform::show_message(
            "MCHOSE Tray",
            "MCHOSE Tray is already running.",
            MessageKind::Info,
        );
        return Ok(());
    }

    logging::write(Level::Info, "starting MCHOSE Tray");
    let window = platform::MessageWindow::new()?;
    let (sender, receiver) = mpsc::channel();
    register_menu_handler(sender.clone());

    let worker = Worker::start(sender);
    let mut ui = TrayUi::new(mode)?;
    ui.paint(Status::Unavailable)?;
    worker.refresh();

    run_event_loop(window, receiver, worker, ui)
}

fn register_menu_handler(sender: mpsc::Sender<Event>) {
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = sender.send(Event::Menu(event));
        platform::wake();
    }));
}

fn run_event_loop(
    window: platform::MessageWindow,
    receiver: mpsc::Receiver<Event>,
    worker: Worker,
    mut ui: TrayUi,
) -> Result<(), String> {
    loop {
        match platform::dispatch_next_message()? {
            false => break,
            true if platform::take_theme_changed() => ui.repaint()?,
            true => {}
        }

        while let Ok(event) = receiver.try_recv() {
            match event {
                Event::Reading(reading) => ui.apply_reading(reading)?,
                Event::Menu(event) => match ui.handle_menu(event)? {
                    Action::None => {}
                    Action::Refresh => worker.refresh(),
                    Action::Quit => window.quit(),
                },
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_line_mode_override_uses_last_mode() {
        let command = parse_command(["--battery".into(), "--percent".into()]).unwrap();
        assert!(matches!(command, Command::Run(DisplayMode::Percentage)));
    }

    #[test]
    fn quit_takes_priority_over_later_arguments() {
        assert!(matches!(
            parse_command(["--quit".into(), "--unknown".into()]).unwrap(),
            Command::Quit
        ));
    }

    #[test]
    fn unknown_option_is_rejected() {
        assert!(parse_command(["--unknown".into()]).is_err());
    }
}
