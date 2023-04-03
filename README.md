# ESC

Extron Scaler Control

I built this tool because the scaler I have does not have aspect ratio control which means all my 4:3 inputs
are stretched to match the output resolution.

This tool monitors the RS232 connection between the host and the scaler and automatically detects when
the scaler's input resolution changes.

After it detects a change:

* It queries for the new input resolution
* Sets the scaled output to match 1:1
* Centers the output

Tested with Extron 300 Series, RGB-HDMI 300 A, it should work with any scaler that implements the SIS protocol
over RS232, I used a USB RS232 adapter (`067b:23a3 Prolific Technology`) and I didn't have any issues.

If you're on Linux, you might need to update the serial port permissions.

* Add your user to the dialout group: `sudo usermod -a -G dialout <username>` on debian/related distros
* or [use udev rules to configure USB tty](https://community.silabs.com/s/article/fixed-tty-device-assignments-in-linux-using-udev)
* or quickly try `sudo chmod 666 /dev/<Your USB device>` but this will be reset on reboot.

#### Usage

`extron <port> <baud> [output_h] [output_v]`

#### Arguments

    <port>        Required: The device path to a serial port
    <baud>        Required: The baud rate to connect at: 300, 600, 1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200
    <output_h>    Optional: The scaler's output horizontal resolution [default: 1920]
    <output_v>    Optional: The scaler's output vertical resolution [default: 1080]

#### Known issues / Improvements

* It assumes that the output resolution is always bigger than the input resolution in both axis.
* It always centers the screen, some people may not want that, e.g. capture card/cropping.
* Theoretically could enter an infinite loop if the serial comms gets corrupted and no line feed is seen.
* Could use a proper state machine

#### Example transcript

    Receiving data on /dev/ttyUSB0 at 9600 baud
    Output resolution: Resolution { h: 1920, v: 1080 }
    Decoding response: Reconfig
    Extron response: Reconfig
    State -> Step: Reconfig, Input size: Hor 0, Ver 0, Output size: Hor 1920, Ver 1080, Waiting: false
    Sending command: APIX
    
    Decoding response: Apix0936
    Extron response: ActivePixels(936)
    State -> Step: GotHorizontalSize, Input size: Hor 936, Ver 0, Output size: Hor 1920, Ver 1080, Waiting: false
    Sending command: ALIN
    
    Decoding response: Alin0250
    Extron response: ActiveLines(250)
    State -> Step: GotVerticalSize, Input size: Hor 936, Ver 250, Output size: Hor 1920, Ver 1080, Waiting: false
    Sending command: 936HSIZ
    
    Decoding response: Hsiz00936
    Extron response: InputHSizeSet
    State -> Step: SetHSize, Input size: Hor 936, Ver 250, Output size: Hor 1920, Ver 1080, Waiting: false
    Sending command: 250VSIZ
    
    Decoding response: Vsiz00250
    Extron response: InputVSizeSet
    State -> Step: SetVSize, Input size: Hor 936, Ver 250, Output size: Hor 1920, Ver 1080, Waiting: false
    Sending command: 10732HCTR
    
    Decoding response: Hctr10732
    Extron response: HorizontalCenter
    State -> Step: SetHCenter, Input size: Hor 936, Ver 250, Output size: Hor 1920, Ver 1080, Waiting: false
    Sending command: 10655VCTR
    
    Decoding response: Vctr10655
    Extron response: VertialCenter
    State -> Step: Uninitialized, Input size: Hor 936, Ver 250, Output size: Hor 1920, Ver 1080, Waiting: false
    Decoding response: Img
    Extron response: Unknown
    State -> Step: Uninitialized, Input size: Hor 936, Ver 250, Output size: Hor 1920, Ver 1080, Waiting: false

#### References

* https://media.extron.com/public/download/files/userman/68-1407-01_RevF_RGB-DVI_HDMI_300.pdf
