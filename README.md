# Modem Handler

🚧 This is currently a proof of concept and is **not stable** 🚧

The modem handler is a service that is intended to act as a wrapper for GSM Modems.

While it was built around the SIM7600 modem, it is should theoretically work with most GSM modems that utilize the [Hayes command set](https://en.wikipedia.org/wiki/Hayes_AT_command_set)


## Before building

For dbus to work, `libdbus-glib-1-dev` is required:

```
sudo apt install libdbus-glib-1-dev
```

## TODO ✅

- [ ] Implement as a DBus service
    - [ ] Expose Rust APIs as DBus API
- [ ] Expose modem features via API
    - [ ] Sending texts
    - [ ] Reading texts
    - [ ] Getting/setting IMEI
    - [ ] Getting/setting SMS format
    - [ ] Getting/setting timezone config
    - [ ] Getting signal quality
    - [ ] Handling of Unsolicited Result Codes (URC)
    - [ ] Getting carrier info
    - [ ] Automatically setting time/timezone
    - [ ] Getting data usage configured?
    - [ ] Calls (answering, hanging up, dialing, etc.)
- [ ] Better error handling (`Box<dyn Error>` prob could be improved)
- [ ] Logging
- [ ] Adding rustdoc strings
- [ ] Proper tests