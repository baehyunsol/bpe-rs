use crate::{Dictionary, DictionaryConfig, construct_dictionary};
use crate::files::merge_files;
use crate::log::write_log;
use crate::utils::prettify_file_size;
use std::sync::mpsc;
use std::thread::{self, sleep};
use std::time::Duration;

pub enum MessageFromMain {
    ReadTheseFiles(Vec<String>),
}

pub enum MessageToMain {
    NewDictionary(Dictionary),
    Done,
}

pub struct Channel {
    tx_from_main: mpsc::Sender<MessageFromMain>,
    rx_to_main: mpsc::Receiver<MessageToMain>,
}

impl Channel {
    pub fn send(&self, msg: MessageFromMain) -> Result<(), mpsc::SendError<MessageFromMain>> {
        self.tx_from_main.send(msg)
    }

    pub fn try_recv(&self) -> Result<MessageToMain, mpsc::TryRecvError> {
        self.rx_to_main.try_recv()
    }

    pub fn block_recv(&self) -> Result<MessageToMain, mpsc::RecvError> {
        self.rx_to_main.recv()
    }
}

pub fn init_channels(n: usize, config: &DictionaryConfig) -> Vec<Channel> {
    (0..n).map(|_| init_channel(config)).collect()
}

pub fn init_channel(config: &DictionaryConfig) -> Channel {
    let (tx_to_main, rx_to_main) = mpsc::channel();
    let (tx_from_main, rx_from_main) = mpsc::channel();
    let config = config.clone();

    thread::spawn(move || {
        event_loop(tx_to_main, rx_from_main, config.clone());
    });

    Channel {
        rx_to_main, tx_from_main
    }
}

pub fn distribute_messages(
    messages: Vec<MessageFromMain>,
    channels: &[Channel],
) -> Result<(), mpsc::SendError<MessageFromMain>> {
    for (index, message) in messages.into_iter().enumerate() {
        channels[index % channels.len()].send(message)?;
    }

    Ok(())
}

pub fn event_loop(
    tx_to_main: mpsc::Sender<MessageToMain>,
    rx_from_main: mpsc::Receiver<MessageFromMain>,
    config: DictionaryConfig,
) {
    let mut queue = vec![];
    let mut got_nothing = 0;
    let worker_id = rand::random::<u32>() & 0xfff_fff;
    let worker_id = format!("worker_{worker_id:06x}");

    write_log(
        config.write_log_at.clone(),
        &worker_id,
        "Hello from worker!",
    );

    loop {
        got_nothing += 1;

        while let Ok(msg) = rx_from_main.try_recv() {
            match msg {
                MessageFromMain::ReadTheseFiles(files) => {
                    queue.push(files);
                    got_nothing = 0;
                },
            }
        }

        while let Some(files) = queue.pop() {
            let files_len = files.len();
            let bytes = merge_files(files, config.dir_option.file_separator);
            write_log(
                config.write_log_at.clone(),
                &worker_id,
                &format!(
                    "registered {files_len} files (total size {})",
                    prettify_file_size(bytes.len() as u64),
                ),
            );

            let new_dictionary = construct_dictionary(&bytes, config.clone());
            write_log(
                config.write_log_at.clone(),
                &worker_id,
                &format!("constructed dictionary with {} words", new_dictionary.len()),
            );

            tx_to_main.send(MessageToMain::NewDictionary(new_dictionary)).unwrap();
            got_nothing = 0;
        }

        sleep(Duration::from_millis(1000));

        if got_nothing > 5 {
            tx_to_main.send(MessageToMain::Done).unwrap();
            write_log(
                config.write_log_at.clone(),
                &worker_id,
                "Goodbye from worker!",
            );
            break;
        }
    }

    drop(tx_to_main)
}
