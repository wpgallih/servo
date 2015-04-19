/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::codegen::Bindings::WebSocketBinding;
use dom::bindings::codegen::Bindings::WebSocketBinding::WebSocketMethods;
use dom::bindings::error::{Error, ErrorResult, Fallible};
use dom::bindings::error::Error::{InvalidState, InvalidAccess};
use dom::bindings::error::Error::{Network, Syntax, Security, Abort, Timeout};
use dom::bindings::global::GlobalRef;
use dom::bindings::js::{Temporary, JSRef};
use dom::bindings::utils::reflect_dom_object;
use dom::eventtarget::{EventTarget, EventTargetTypeId};
use util::str::DOMString;

use websocket::{Message, Sender, Receiver};
use websocket::client::request::Url;
use websocket::Client;
use std::cell::Cell;

#[derive(PartialEq, Copy)]
#[jstraceable]
enum WebsocketRequestState {
	Unsent = 0,
	Opened = 1,
	Sending = 2,
	Receiving = 3,
	Closing = 4,
	Closed = 5,
}

#[dom_struct]
pub struct WebSocket {
    eventtarget: EventTarget,
    url: DOMString,
	ready_state: Cell<WebsocketRequestState>
}

impl WebSocket {
    pub fn new_inherited(url: DOMString) -> WebSocket {
        println!("Creating websocket...");
	let copied_url = url.clone();
	WebSocket::Open(copied_url);
	WebSocket {
            eventtarget: EventTarget::new_inherited(EventTargetTypeId::WebSocket),
            url: url,
		ready_state: Cell::new(WebsocketRequestState::Unsent)
        }

    }

    pub fn new(global: GlobalRef, url: DOMString) -> Temporary<WebSocket> {
        reflect_dom_object(box WebSocket::new_inherited(url),
                           global,
                           WebSocketBinding::Wrap)
    }

    pub fn Constructor(global: GlobalRef, url: DOMString) -> Fallible<Temporary<WebSocket>> {
        Ok(WebSocket::new(global, url))
    }
    fn Open(url: DOMString) -> ErrorResult {
    	println!("Trying to connect.");
	let parsed_url = Url::parse(url.as_slice()).unwrap();
   	let request = Client::connect(parsed_url).unwrap();
	let response = request.send().unwrap();
	response.validate().unwrap();
	println!("Successful connection.");
	Ok(())
    }
	
	fn send(self) -> ErrorResult {
		tx_1 = self.tx.clone();
		let send_loop = thread::scoped(move || {
			loop {
				let message = match self.rx.recv() {
					Ok(m) => m,
					Err(e) => {
						println!("Send loop: {:?}",e);
						return;
					}
				};
				match message {
					Message::Close(_) => {
						let _ = self.sender.send_message(message);
						// If it's a close message, send it and return
						return;
					}
					_ => (),
				}
				//Send the message
				match self.sender.send_message(message){
					Ok(()) => (),
					Err(e) => {
						println!("Send Loop: {:?}", e);
						let _ = self.sender.send_message(Message::Close(None));
						return;
					}
				}
			}	
		});
	}
}

impl<'a> WebSocketMethods for JSRef<'a, WebSocket> {
    fn Url(self) -> DOMString {
	println!("Cloning URL");
       self.url.clone()
    }
	
   fn ReadyState(self) -> u16 {
   	println!("Setting readystate");
	self.ready_state.get() as u16
   }

   fn Open (self) -> ErrorResult {
	println!("Trying to connect.");
	let parsed_url = Url::parse(self.url.as_slice()).unwrap();
   	let request = Client::connect(parsed_url).unwrap();
	let response = request.send().unwrap();
	response.validate().unwrap();
	println!("Successful connection.");
	Ok(())
   }
}
