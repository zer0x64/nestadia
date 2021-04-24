import React, { createRef, RefObject } from "react";
import RGB_VALUE_TABLE from "./RGB_VALUES_TABLE";

class Emulator extends React.Component {
    canvasRef: RefObject<HTMLCanvasElement>;

    constructor(props: any) {
        super(props);
        this.canvasRef = createRef();
    }

    componentDidMount() {
        let canvas = this.canvasRef?.current;
        if (canvas) {
            canvas.width = 256;
            canvas.height = 240;
        }

        let ws = new WebSocket("ws://" + window.location.host + "/api/emulator/rom1");
        ws.binaryType = 'arraybuffer'

        ws.addEventListener("message", (event) => {
            let frame: Uint8Array = new Uint8Array(event.data);
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

    render() {
        return (<canvas ref={this.canvasRef}></canvas>)
    }
}

export default Emulator;