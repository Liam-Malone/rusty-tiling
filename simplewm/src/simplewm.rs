use xcb::{
    x,
    Xid,
};

/* I'm just figuring all this shit out.. 
 * it's all new to me
 * this will take a while
 */

//basic setup code yoinked from xcb docs, 
//with tweaks to comments so I can better understand
fn main() -> xcb::Result<()> {
    //connects to X server
    let (conn, screen_num) = xcb::Connection::connect(None)?;

    let setup = conn.get_setup();
    let screen = setup.roots().nth(screen_num as usize).unwrap();

    // Generate Xid for client window
    // type inference necessary
    let window: x::Window = conn.generate_id();

    // create window - pass request object - receive cookie to check success
    let cookie = conn.send_request_checked(&x::CreateWindow {
        depth: x::COPY_FROM_PARENT as u8,
        wid: window,
        parent: screen.root(),
        x: 0,
        y: 0,
        width: 150,
        height: 150,
        border_width: 1,
        class: x::WindowClass::InputOutput,
        visual: screen.root_visual(),
        // list must be in same order as 'Cw' enum
        value_list: &[
               x::Cw::BackPixel(screen.white_pixel()),
               x::Cw::EventMask(x::EventMask::EXPOSURE | x::EventMask::KEY_PRESS),
        ],
    });

    //check if win creation worked
    conn.check_request(cookie)?;

    //now show window (x calls it "map")
    //no success check, discard cookie
    conn.send_request(&x::MapWindow {
        window,
    });

    //need atoms for app, send some reqs and await replies
    let (wm_protocols, wm_del_window, wm_state, wm_state_maxv, wm_state_maxh) = {
        let cookies = (
            conn.send_request(&x::InternAtom {
                only_if_exists: true,
                name: b"WM_PROTOCOLS",
            }),
            conn.send_request(&x::InternAtom {
                only_if_exists: true,
                name: b"WM_DELETE_WINDOW",
            }),
            conn.send_request(&x::InternAtom {
                only_if_exists: true,
                name: b"_NET_WM_STATE",
            }),
            conn.send_request(&x::InternAtom {
                only_if_exists: true,
                name: b"_NET_WM_STATE_MAXIMIZED_VERT",
            }),
            conn.send_request(&x::InternAtom {
                only_if_exists: true,
                name: b"_NET_WM_STATE_MAXIMIZED_HORZ",
            }),
            );
        (
            conn.wait_for_reply(cookies.0)?.atom(),
            conn.wait_for_reply(cookies.1)?.atom(),
            conn.wait_for_reply(cookies.2)?.atom(),
            conn.wait_for_reply(cookies.3)?.atom(),
            conn.wait_for_reply(cookies.4)?.atom(),
        )
    };

    //activate window close event
    //can click x, but that closes loop by connection shutdown err
    conn.check_request(conn.send_request_checked(&x::ChangeProperty {
        mode: x::PropMode::Replace,
        window,
        property: wm_protocols,
        r#type: x::ATOM_ATOM,
        data: &[wm_del_window],
    }))?;

    //prev req checked, flush unnecessary, otherwise flush with:
    //  conn.flush()?;

    let mut maximized = false;

    //enter main event loop
    
    loop {
        match conn.wait_for_event()? {
            xcb::Event::X(x::Event::KeyPress(ev)) => {
                if ev.detail() == 0x3a {
                    // M key pressed (qwerty)

                    /* keymap support managed with 'xkb' extension
                     * and 'xkbcommon-rs' crate
                     */

                    let data = x::ClientMessageData::Data32([
                        if maximized { 0 } else { 1 },
                        wm_state_maxv.resource_id(),
                        wm_state_maxh.resource_id(),
                        0,
                        0,
                    ]);
                    let event = x::ClientMessageEvent::new(window, wm_state, data);
                    let cookie = conn.send_request_checked(&x::SendEvent {
                        propagate: false,
                        destination: x::SendEventDest::Window(screen.root()),
                        event_mask: x::EventMask::STRUCTURE_NOTIFY,
                        event: &event,
                    });
                    conn.check_request(cookie)?;

                    //as before, failing to check for error = need to flush

                    maximized = !maximized;
                } else if ev.detail() == 0x18 {
                    // Q (on qwerty)
                    // exit event loop and end program
                    
                    break Ok(());
                }
            }

            xcb::Event::X(x::Event::ClientMessage(ev)) => {
                //we have message received from server
                if let x::ClientMessageData::Data32([atom, ..]) = ev.data() {
                    if atom == wm_del_window.resource_id() {
                        // received atom is 'WM_DELETE_WINDOW'
                        // could add check here if user needs to save
                        // or just end right away
                        break Ok(());
                    }
                }
            }

            _ => {}
        }
    }
}
