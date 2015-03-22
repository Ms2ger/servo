/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use resource_task::{ProgressMsg, Metadata, LoadData, start_sending, TargetedLoadResponse, ResponseSenders};
use resource_task::ProgressMsg::{Payload, Done};

use std::borrow::ToOwned;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use util::task::spawn_named;

fn read_all(reader: &mut File, progress_chan: &Sender<ProgressMsg>)
        -> Result<(), String> {
    let mut buf = vec!();
    match reader.read_to_end(&mut buf) {
        Ok(_) => Ok(progress_chan.send(Payload(buf)).unwrap()),
        Err(e) => Err(e.description().to_string()),
    }
}

pub fn factory(load_data: LoadData, start_chan: Sender<TargetedLoadResponse>) {
    let url = load_data.url;
    assert!(&*url.scheme == "file");
    let senders = ResponseSenders {
        immediate_consumer: start_chan,
        eventual_consumer: load_data.consumer,
    };
    let progress_chan = start_sending(senders, Metadata::default(url.clone()));
    spawn_named("file_loader".to_owned(), move || {
        let file_path: Result<PathBuf, ()> = url.to_file_path();
        match file_path {
            Ok(file_path) => {
                match File::open(&file_path) {
                    Ok(ref mut reader) => {
                        let res = read_all(reader, &progress_chan);
                        progress_chan.send(Done(res)).unwrap();
                    }
                    Err(e) => {
                        let message = Done(Err(e.description().to_string()));
                        progress_chan.send(message).unwrap();
                    }
                }
            }
            Err(_) => {
                progress_chan.send(Done(Err(url.to_string()))).unwrap();
            }
        }
    });
}
