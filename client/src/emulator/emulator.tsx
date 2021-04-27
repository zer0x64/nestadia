import React, { ChangeEvent, createRef, RefObject } from "react";
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
            ws.send(fileBuffer);
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
        if (canvas) {
            canvas.width = 256;
            canvas.height = 240;
        }
    }

    render() {
        let content;
        if(this.state.started) {
            content = (<canvas ref={this.canvasRef} onLoad={this.onCanvasLoad}></canvas>)
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