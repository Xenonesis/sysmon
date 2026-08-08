pub(crate) mod commands;
pub(crate) mod events;

use std::sync::mpsc::{self, Receiver, Sender};

use commands::{ActionCommand, MonitoringCommand};
use events::AppEvent;

pub(crate) struct AppChannels {
    pub monitoring_sender: Sender<MonitoringCommand>,
    pub action_sender: Sender<ActionCommand>,
    pub event_sender: Sender<AppEvent>,
    pub event_receiver: Receiver<AppEvent>,
    pub monitoring_receiver: Option<Receiver<MonitoringCommand>>,
    pub action_receiver: Option<Receiver<ActionCommand>>,
}

impl AppChannels {
    pub fn new() -> Self {
        let (monitoring_sender, monitoring_receiver) = mpsc::channel();
        let (action_sender, action_receiver) = mpsc::channel();
        let (event_sender, event_receiver) = mpsc::channel();
        Self {
            monitoring_sender,
            action_sender,
            event_sender,
            event_receiver,
            monitoring_receiver: Some(monitoring_receiver),
            action_receiver: Some(action_receiver),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monitoring_channel_delivers_shutdown() {
        let mut channels = AppChannels::new();
        channels.monitoring_sender.send(MonitoringCommand::Shutdown).unwrap();
        let command = channels.monitoring_receiver.take().unwrap().recv().unwrap();
        assert!(matches!(command, MonitoringCommand::Shutdown));
    }
}
