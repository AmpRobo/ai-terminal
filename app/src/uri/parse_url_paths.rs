use url::Url;

use crate::ChannelState;
use crate::cloud_object::extract_server_id_and_object_type_from_ai_terminal_drive_link;
use crate::drive::OpenAiTerminalDriveObjectArgs;

#[derive(PartialEq, Debug)]
pub enum AiTerminalWebLink {
    Session,
    DriveObject(Box<OpenAiTerminalDriveObjectArgs>),
}

pub fn get_item_data_from_ai_terminal_link(url: &Url) -> Option<AiTerminalWebLink> {
    if url.origin() == ChannelState::server_root_domain() {
        url.path_segments().and_then(|mut path_segments| {
            path_segments.next().and_then(|segment| match segment {
                "drive" => extract_server_id_and_object_type_from_ai_terminal_drive_link(url)
                    .map(|args| AiTerminalWebLink::DriveObject(Box::new(args))),
                "session" => Some(AiTerminalWebLink::Session),
                _ => None,
            })
        })
    } else {
        None
    }
}
