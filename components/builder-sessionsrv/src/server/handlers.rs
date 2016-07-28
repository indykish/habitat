// Copyright (c) 2016 Chef Software Inc. and/or applicable contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use dbcache::{self, ExpiringSet, InstaSet};
use hab_net::server::Envelope;
use protocol::net::{self, ErrCode, NetOk};
use protocol::sessionsrv as proto;
use zmq;

use super::ServerState;
use error::Result;
use privilege;

pub fn account_get(req: &mut Envelope,
                   sock: &mut zmq::Socket,
                   state: &mut ServerState)
                   -> Result<()> {
    let msg: proto::AccountGet = try!(req.parse_msg());
    match state.datastore.accounts.find_by_username(&msg.get_name().to_string()) {
        Ok(account) => try!(req.reply_complete(sock, &account)),
        Err(dbcache::Error::EntityNotFound) => {
            let err = net::err(ErrCode::ENTITY_NOT_FOUND, "ss:account_get:0");
            try!(req.reply_complete(sock, &err));
        }
        Err(e) => {
            error!("datastore error, err={:?}", e);
            let err = net::err(ErrCode::INTERNAL, "ss:account_get:1");
            try!(req.reply_complete(sock, &err));
        }
    }
    Ok(())
}

pub fn account_search(req: &mut Envelope,
                      sock: &mut zmq::Socket,
                      state: &mut ServerState)
                      -> Result<()> {
    let mut msg: proto::AccountSearch = try!(req.parse_msg());
    let result = match msg.get_key() {
        proto::AccountSearchKey::Id => {
            let value: u64 = msg.take_value().parse().unwrap();
            state.datastore.accounts.find(&value)
        }
        proto::AccountSearchKey::Name => {
            state.datastore.accounts.find_by_username(&msg.take_value())
        }
    };
    match result {
        Ok(account) => try!(req.reply_complete(sock, &account)),
        Err(dbcache::Error::EntityNotFound) => {
            let err = net::err(ErrCode::ENTITY_NOT_FOUND, "ss:account-search:0");
            try!(req.reply_complete(sock, &err));
        }
        Err(e) => {
            error!("datastore error, err={:?}", e);
            let err = net::err(ErrCode::INTERNAL, "ss:account-search:1");
            try!(req.reply_complete(sock, &err));
        }
    }
    Ok(())
}

pub fn grant_flags(req: &mut Envelope,
                   sock: &mut zmq::Socket,
                   state: &mut ServerState)
                   -> Result<()> {
    let msg: proto::GrantFlagToTeams = try!(req.parse_msg());
    for team in msg.get_teams() {
        try!(privilege::associate_team(&state.datastore, msg.get_flag(), *team));
    }
    try!(req.reply_complete(sock, &NetOk::new()));
    Ok(())
}

pub fn grant_list(req: &mut Envelope,
                  sock: &mut zmq::Socket,
                  state: &mut ServerState)
                  -> Result<()> {
    let msg: proto::ListFlagGrants = try!(req.parse_msg());
    Ok(())
}

pub fn session_create(req: &mut Envelope,
                      sock: &mut zmq::Socket,
                      state: &mut ServerState)
                      -> Result<()> {
    let mut msg: proto::SessionCreate = try!(req.parse_msg());
    let account: proto::Account = match state.datastore
        .sessions
        .find(&msg.get_token().to_string()) {
        Ok(session) => state.datastore.accounts.find(&session.get_owner_id()).unwrap(),
        _ => try!(state.datastore.accounts.find_or_create(&msg)),
    };
    let mut session_token = proto::SessionToken::new();
    session_token.set_token(msg.take_token());
    session_token.set_owner_id(account.get_id());
    session_token.set_provider(msg.get_provider());
    // JW TODO: handle database error & return net error case
    try!(state.datastore.sessions.write(&mut session_token));
    let mut session: proto::Session = account.into();
    session.set_token(session_token.take_token());
    // JW TODO: handle this and reply with a partial auth (sans features) if they can't be obtained
    try!(privilege::set_features(&state.datastore, &state.github, &mut session));
    try!(req.reply_complete(sock, &session));
    Ok(())
}

pub fn session_get(req: &mut Envelope,
                   sock: &mut zmq::Socket,
                   state: &mut ServerState)
                   -> Result<()> {
    let msg: proto::SessionGet = try!(req.parse_msg());
    match state.datastore.sessions.find(&msg.get_token().to_string()) {
        Ok(mut token) => {
            let account: proto::Account =
                state.datastore.accounts.find(&token.get_owner_id()).unwrap();
            let mut session: proto::Session = account.into();
            session.set_token(token.take_token());
            // JW TODO: handle this and reply with a partial auth (sans features) if they can't be
            // obtained
            try!(privilege::set_features(&state.datastore, &state.github, &mut session));
            try!(req.reply_complete(sock, &session));
        }
        Err(dbcache::Error::EntityNotFound) => {
            let err = net::err(ErrCode::SESSION_EXPIRED, "ss:auth:4");
            try!(req.reply_complete(sock, &err));
        }
        Err(e) => {
            error!("datastore error, err={:?}", e);
            let err = net::err(ErrCode::INTERNAL, "ss:auth:5");
            try!(req.reply_complete(sock, &err));
        }
    }
    Ok(())
}
