use nix::unistd::{self, ForkResult};
use std::fs::{create_dir_all, metadata};
use std::os::unix::prelude::*;

use crate::container;
use crate::dotcntr;
use crate::fs;
use crate::ipc;
use crate::procfs;
use crate::sys_ext::Passwd;
use crate::types::{Error, Result};
use crate::user_namespace::IdMap;

mod child;
mod parent;

pub struct AttachOptions {
    pub command: Option<String>,
    pub arguments: Vec<String>,
    pub container_name: String,
    pub container_types: Vec<Box<dyn container::Container>>,
    pub effective_user: Option<Passwd>,
}

pub fn attach(opts: &AttachOptions) -> Result<()> {
    let container_pid = tryfmt!(
        container::lookup_container_pid(opts.container_name.as_str(), &opts.container_types),
        ""
    );

    let (uid_map, gid_map) = tryfmt!(
        IdMap::new_from_pid(container_pid),
        "failed to read usernamespace properties of {}",
        container_pid
    );

    let metadata = tryfmt!(
        metadata(procfs::get_path().join(container_pid.to_string())),
        "failed to container uid/gid"
    );

    let mut home = None;
    let mut effective_uid = None;
    let mut effective_gid = None;
    let container_uid = unistd::Uid::from_raw(uid_map.map_id_up(metadata.uid()));
    let container_gid = unistd::Gid::from_raw(gid_map.map_id_up(metadata.gid()));

    if let Some(ref passwd) = opts.effective_user {
        effective_uid = Some(passwd.pw_uid);
        effective_gid = Some(passwd.pw_gid);
        home = Some(passwd.pw_dir.as_ref());
    }

    let process_status = tryfmt!(
        procfs::status(container_pid),
        "failed to get status of target process"
    );

    let dotcntr = tryfmt!(dotcntr::create(&process_status), "failed to setup /.cntr");

    let cntrfs = tryfmt!(
        fs::CntrFs::new(
            &fs::CntrMountOptions {
                prefix: "/",
                uid_map,
                gid_map,
                effective_uid,
                effective_gid,
            },
            Some(dotcntr),
        ),
        "cannot mount filesystem"
    );

    tryfmt!(
        create_dir_all("/var/lib/cntr"),
        "failed to create /var/lib/cntr"
    );

    let (parent_sock, child_sock) = tryfmt!(ipc::socket_pair(), "failed to set up ipc");

    let res = unsafe { unistd::fork() };
    match tryfmt!(res, "failed to fork") {
        ForkResult::Parent { child } => parent::run(child, &parent_sock, cntrfs),
        ForkResult::Child => {
            let child_opts = child::ChildOptions {
                command: opts.command.clone(),
                arguments: opts.arguments.clone(),
                mount_ready_sock: &child_sock,
                uid: container_uid,
                gid: container_gid,
                fs: cntrfs,
                process_status,
                home,
            };
            child::run(&child_opts)
        }
    }
}
