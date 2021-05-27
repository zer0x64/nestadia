import React from 'react';
import Button from '@material-ui/core/Button'
import AppState from '../appstate';
import EmulatorMode from '../emulator/emulatorMode';

class MainPage extends React.Component<{setAppState: Function, setEmulatorMode: Function}, {}> {
    constructor(props: any) {
        super(props)
    }

    componentDidMount() {
    }

    render() {
        var linkSize = {fontSize: "0.5em"}
        return (
            <div>
                <p>Welcome to Nestadia!</p>
                <Button variant="contained" color="secondary" onClick={() => {this.props.setEmulatorMode(EmulatorMode.Normal); this.props.setAppState(AppState.EmulatorPage)}}>Try the emulator with a free ROM!</Button>
                <Button variant="contained" color="primary" onClick={() => {this.props.setEmulatorMode(EmulatorMode.Custom); this.props.setAppState(AppState.EmulatorPage)}}>Try the emulator with your own ROM!</Button><br></br>
            </div>
        )
    }
}

export default MainPage;
