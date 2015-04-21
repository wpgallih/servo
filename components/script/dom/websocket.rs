/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::codegen::Bindings::WebSocketBinding;
use dom::bindings::codegen::Bindings::WebSocketBinding::WebSocketMethods;
use dom::bindings::codegen::Bindings::EventHandlerBinding::EventHandlerNonNull;
use dom::bindings::codegen::InheritTypes::EventTargetCast;
use dom::bindings::error::{Error, ErrorResult, Fallible};
use dom::bindings::error::Error::{InvalidState, InvalidAccess};
use dom::bindings::error::Error::{Network, Syntax, Security, Abort, Timeout};
use dom::event::{Event, EventBubbles, EventCancelable, EventHelpers};
use dom::bindings::global::{GlobalField, GlobalRef, GlobalRoot};
use dom::bindings::js::{Temporary, JSRef};
use dom::bindings::utils::reflect_dom_object;
use dom::eventtarget::{EventTarget, EventTargetHelpers, EventTargetTypeId};
use util::str::DOMString;
use script_task::Runnable;
use script_task::ScriptMsg;
use dom::bindings::refcounted::Trusted;

use websocket::{Message, Sender, Receiver};
use websocket::client::request::Url;
use websocket::Client;
use std::cell::Cell;
use std::borrow::ToOwned;
use std::sync::mpsc::channel;

#[derive(PartialEq, Copy)]
#[jstraceable]
enum WebSocketRequestState {
    Connecting = 0,
    Open = 1,
    Closing = 2,
    Closed = 3,
}

#[dom_struct]
pub struct WebSocket {
    eventtarget: EventTarget,
    url: DOMString,
    global: GlobalField,
	ready_state: Cell<WebSocketRequestState>
}

impl WebSocket {
    pub fn new_inherited(global: GlobalRef, url: DOMString) -> WebSocket {
        println!("Creating websocket...");
	let copied_url = url.clone();
	WebSocket {
            eventtarget: EventTarget::new_inherited(EventTargetTypeId::WebSocket),
            url: url,
            global: GlobalField::from_rooted(&global),
		ready_state: Cell::new(WebSocketRequestState::Connecting)
        }

    }

    pub fn new(global: GlobalRef, url: DOMString) -> Temporary<WebSocket> {
        let ws_root = reflect_dom_object(box WebSocket::new_inherited(global, url),
                           global,
                           WebSocketBinding::Wrap).root();
        ws_root.r().Open();
        Temporary::from_rooted(ws_root.r())
    }

    pub fn Constructor(global: GlobalRef, url: DOMString) -> Fallible<Temporary<WebSocket>> {
    	Ok(WebSocket::new(global, url))
    }
	
	/*fn send(self) -> ErrorResult {
		 let tx_1 = self.tx.clone();
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
	}*/
}

impl<'a> WebSocketMethods for JSRef<'a, WebSocket> {
	event_handler!(open, GetOnopen, SetOnopen);
	event_handler!(close, GetOnclose, SetOnclose);
	event_handler!(error, GetOnerror, SetOnerror);

    fn Url(self) -> DOMString {
	println!("Cloning URL");
       self.url.clone()
    }
	
   fn ReadyState(self) -> u16 {
   	println!("Setting readystate");
	self.ready_state.get() as u16
   }

   fn Open(self) -> ErrorResult {
   		let global_root = self.global.root();
        let addr: Trusted<WebSocket> = Trusted::new(global_root.r().get_cx(), self, global_root.r().script_chan().clone()); 
    	let open_task = box WebSocketTaskHandler::new(addr.clone(), WebSocketTask::Open);
        global_root.r().script_chan().send(ScriptMsg::RunnableMsg(open_task)).unwrap();
   		Ok(())
    }

    fn Close(self, code: Option<u16>, reason: Option<DOMString>) -> Fallible<()>{
	if(code.is_some()){ //Code defined
	   if(!(code==Some(1000)||
		(code>=Some(3000) && code<=Some(4999))
	       )
	   )
	   { //Check code is NOT 1000 NOR in the range of 3000-4999 (inclusive)
		return Err(Error::InvalidAccess); //Throw InvalidAccessError and abort
	   }
	}
	if(reason.is_some()){ //reason defined
	   if(reason.unwrap().as_bytes().len() > 123) //reason cannot be larger than 123 bytes
	   {
		return Err(Error::Syntax); //Throw SyntaxError and abort
	   }
	}
	
	match self.ready_state.get() { //Returns the value of the cell
	   WebSocketRequestState::Closing => {} //Do nothing
	   WebSocketRequestState::Closed => {} //Do nothing
	   WebSocketRequestState::Connecting => { //Connection is not yet established
		/*By setting the state to closing, the open function
		  will attempt to close the websocket*/
		self.ready_state.set(WebSocketRequestState::Closing); //Set state to closing
	   }
	   WebSocketRequestState::Open => {//Closing handshake not started - still in open

		//Send a close task
		let global_root = self.global.root();
      		let addr: Trusted<WebSocket> = Trusted::new(global_root.r().get_cx(), self, global_root.r().script_chan().clone());
	    	let close_task = box WebSocketTaskHandler::new(addr.clone(), WebSocketTask::Close);
		global_root.r().script_chan().send(ScriptMsg::RunnableMsg(close_task)).unwrap();

	      //Start the Websocket closing handshake - how? What does this really mean for the websocket object?
	      //if code.is_some - WebSocket status code in close message to be the same as code
	      //if reason.is_some - Websocket close message reason to be same as reason
	   }
	   //_ => { self.ready_state.set(WebSocketRequestState::Closing); } //Unreachable - Uncomment if you add more states to WebSocketRequestState
	}
	return Err(Error::Syntax); //Throw SyntaxError and abort
    }
}


pub enum WebSocketTask {
    Open,
    Close,
}

pub struct WebSocketTaskHandler {
    addr: Trusted<WebSocket>,
    task: WebSocketTask,
}

impl WebSocketTaskHandler {
    pub fn new(addr: Trusted<WebSocket>, task: WebSocketTask) -> WebSocketTaskHandler {
        WebSocketTaskHandler {
            addr: addr,
            task: task,
        }
    }

    fn dispatch_open(&self) {
    	println!("Trying to connect.");
    	let ws = self.addr.to_temporary().root();
		let parsed_url = Url::parse(ws.r().url.as_slice()).unwrap();
   		let request = Client::connect(parsed_url).unwrap();
		let response = request.send().unwrap();
		response.validate().unwrap();
		println!("Successful connection.");
		//TODO: Check to see if ready_state is Closing - means we're trying to fail the websocket
		// if so, call the closing method and return (don't set any states - should be in closed state)
    	let global = ws.r().global.root();
        let event = Event::new(global.r(),
                               "open".to_owned(),
                               EventBubbles::DoesNotBubble,
                               EventCancelable::Cancelable).root();
        let target: JSRef<EventTarget> = EventTargetCast::from_ref(ws.r());
        event.r().fire(target);
        println!("Fired event.");
    }
    fn dispatch_close(&self) {
    	println!("Trying to close.");
    }
}

impl Runnable for WebSocketTaskHandler {
    fn handler(self: Box<WebSocketTaskHandler>) {
        match self.task {
            WebSocketTask::Open => {
                self.dispatch_open();
            }
            WebSocketTask::Close => {
                self.dispatch_close();
            }
        }
    }
}

