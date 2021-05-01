import React, { ChangeEvent, CSSProperties, createRef, RefObject } from "react";
import RGB_VALUE_TABLE from "./RGB_VALUES_TABLE";
import * as gzip from 'gzip-js';
import EmulatorMode from "./emulatorMode";

class Emulator extends React.Component<{setAppState: Function, mode: EmulatorMode}, {started: boolean, roms: string[], controller: number}> {
    canvasRef: RefObject<HTMLCanvasElement>;
    websocket: WebSocket | undefined;

    constructor(props: any) {
        super(props);
        this.canvasRef = createRef();
        this.state = {started: false, roms: [], controller: 0}
        this.onFileChangeHandler = this.onFileChangeHandler.bind(this);
        this.onListChangeHandler = this.onListChangeHandler.bind(this);
        this.onCanvasLoad = this.onCanvasLoad.bind(this);
    }

    async componentDidMount() {
        if (this.props.mode == EmulatorMode.Normal) {
            let roms: string[] = await (await fetch("/api/list")).json()
            this.setState({roms: roms});
        }
        else if(this.props.mode == EmulatorMode.Dev) {
            let ws = this.startEmulation("/api/dev/emulator");
            ws.onopen = (e) => {
                this.wsAddEventListener(ws);
                this.websocket = ws;
                this.controllerAddEventListener();
                this.setState({started: true});
            }
        }
    }

    startEmulation(path: string): WebSocket {
        let ws = new WebSocket("ws://" + window.location.host + path);
        ws.binaryType = 'arraybuffer'

        return ws;
    }

    wsAddEventListener(ws: WebSocket) {
        ws.addEventListener("message", (event) => {
            let frameEncoded: Uint8Array = new Uint8Array(event.data);
            let frame = gzip.unzip(frameEncoded);

            let ctx = this.canvasRef.current?.getContext("2d");

            if (ctx) {
                ctx.clearRect(0, 0, 256, 240);

                let image = ctx.createImageData(256, 240)
                for(let i = 0; i < 256 * 240; i++) {
                    let rgb = RGB_VALUE_TABLE[frame[i]];
                    image.data[i * 4] = rgb[0];
                    image.data[i * 4 + 1] = rgb[1];
                    image.data[i * 4 + 2] = rgb[2];
                    image.data[i * 4 + 3] = 255;
                };

                ctx.putImageData(image, 0, 0);
            }
        })
    }

    controllerAddEventListener() {
        document.addEventListener("keydown", (e) => {
            let current = this.state.controller;
            if(e.code == "KeyX") {
                current |= 0x80;
            };
            if(e.code == "KeyZ") {
                current |= 0x40;
            };
            if(e.code == "KeyA") {
                current |= 0x20;
            };
            if(e.code == "KeyS") {
                current |= 0x10;
            };
            if(e.code == "ArrowUp") {
                current |= 0x08;
            };
            if(e.code == "ArrowDown") {
                current |= 0x04;
            };
            if(e.code == "ArrowLeft") {
                current |= 0x02;
            };
            if(e.code == "ArrowRight") {
                current |= 0x01;
            };

            this.setState({controller: current});
            this.websocket?.send(new Uint8Array([current]));
        }, false)
        document.addEventListener("keyup", (e) => {
            let current = this.state.controller;
            if(e.code == "KeyX") {
                current &= ~0x80;
            };
            if(e.code == "KeyZ") {
                current &= ~0x40;
            };
            if(e.code == "KeyA") {
                current &= ~0x20;
            };
            if(e.code == "KeyS") {
                current &= ~0x10;
            };
            if(e.code == "ArrowUp") {
                current &= ~0x08;
            };
            if(e.code == "ArrowDown") {
                current &= ~0x04;
            };
            if(e.code == "ArrowLeft") {
                current &= ~0x02;
            };
            if(e.code == "ArrowRight") {
                current &= ~0x01;
            };

            this.setState({controller: current});
            this.websocket?.send(new Uint8Array([current]));
        }, false)
    }

    componentWillUnmount() {
        this.websocket?.close();
    }

    async onFileChangeHandler(event: any) {
        let file: File = event.target.files[0];
        let fileBuffer = new Uint8Array(await file.arrayBuffer())

        let ws: WebSocket = this.startEmulation("/api/emulator/custom");
        
        ws.onopen = (e) => {
            let romLength = fileBuffer.length;
            let lengthBuf = new Uint8Array(4);
            lengthBuf[0] = (romLength & 0x000000FF) >> 0;
            lengthBuf[1] = (romLength & 0x0000FF00) >> 8;
            lengthBuf[2] = (romLength & 0x00FF0000) >> 16;
            lengthBuf[3] = (romLength & 0xFF000000) >> 24;

            // Send ROM by chunk
            let chunkStart = 0;
            let chunkEnd = 0;

            do {
                chunkEnd = Math.min(romLength, chunkEnd + 50000);

                if (chunkStart == 0) {
                    let chunk = new Uint8Array(chunkEnd + 4);
                    chunk.set(lengthBuf);
                    chunk.set(fileBuffer.subarray(chunkStart, chunkEnd), 4);
                    ws.send(chunk);
                } else {
                    ws.send(fileBuffer.subarray(chunkStart, chunkEnd));
                }
                chunkStart = chunkEnd;
            } while (chunkEnd < romLength);

            this.wsAddEventListener(ws);
            this.websocket = ws;
            this.controllerAddEventListener();
            this.setState({started: true});
        }
    }

    onListChangeHandler(event: any) {
        if(event.target.value) {
            let ws: WebSocket = this.startEmulation("/api/emulator/" + event.target.value);
            ws.onopen = (e) => {
                this.wsAddEventListener(ws);
                this.websocket = ws;
                this.controllerAddEventListener();
                this.setState({started: true});
            }
        }
    }

    onCanvasLoad(event: any) {
        let canvas = this.canvasRef?.current;
    }

    render() {
        let content;
        if(this.state.started) {
            const keybindStyle: CSSProperties = {
                border: "1px solid white",
                fontSize: "calc(7px + 1vmin)",
                textAlign: "left",
                width: "60vh",
            }

            const horizontalAlign: CSSProperties = {
                display: 'inline-flex',
                padding: "0, 0, 0, 0",
            }

            let canvasWidth = window.screen.height * 0.5
            const canvasStyle: CSSProperties = {
                width: window.screen.height * 0.5,
                height: window.screen.height * 0.5 * (240/256),
            }

            content = (
              <div style={horizontalAlign}>
                <canvas width="256" height="240" style={canvasStyle} ref={this.canvasRef} onLoad={this.onCanvasLoad}></canvas>
                <div style={keybindStyle}>
                  <h3>Keybind</h3>
                  <p>Arrows =&gt; D-pad<br/>
                    X =&gt; A<br/>
                    Z =&gt; B<br/>
                    A =&gt; Select<br/>
                    S =&gt; Start<br/>
                  </p>
                </div>
              </div>
            )
        }
        else if(this.props.mode == EmulatorMode.Normal) {
            let choices: any[] = [];
            this.state.roms.forEach(element => {
                choices.push((<option value={element}>{element}</option>))
            });

            content = (<select onChange={this.onListChangeHandler}><option>Please select a ROM</option>{choices}</select>);
        }
        else if (this.props.mode == EmulatorMode.Custom) {
            content = (<input type="file" name="file" onChange={this.onFileChangeHandler}/>)
        }
        else {
            content = (<div></div>)
        }

        return content;
    }
}

export default Emulator;
