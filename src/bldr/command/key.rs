//
// Copyright:: Copyright (c) 2015 Chef Software, Inc.
// License:: Apache License, Version 2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

use std::fs;
use util::http;
use util::gpg;
use config::Config;
use error::{BldrResult};

pub fn install(config: &Config) -> BldrResult<()> {
    try!(fs::create_dir_all("/opt/bldr/cache/keys"));
    let filename = try!(http::download_key(&config.key(), &config.url(), "/opt/bldr/cache/keys"));
    try!(gpg::import("key", &filename));
    Ok(())
}
