import React from 'react';
import Button from '@material-ui/core/Button'
import AppState from '../appstate';
import EmulatorMode from '../emulator/emulatorMode';

class DevDashboard extends React.Component<{setAppState: Function, setEmulatorMode: Function}, {flag: string}> {
    constructor(props: any) {
        super(props)

        this.state = {flag: ""};
    }

    async componentDidMount() {
        this.setState({flag: await this.getFlag()});
    }

    async getFlag(): Promise<string> {
        let flagRequest = await fetch("/api/dev/flag");
    
        if (flagRequest.ok) {
          return await flagRequest.text();
        }
        else {
          return "";
        }
      }

    render() {
        return (
            <div>
                <p>{this.state.flag}</p>
                <Button id="redirectToChatroom" variant="contained" color="secondary" onClick={() => {this.props.setEmulatorMode(EmulatorMode.Dev); this.props.setAppState(AppState.EmulatorPage)}}>Test out the new game</Button>
                <Button id="redirectToLogin" variant="contained" color="primary" href="/api/dev/debug_build">Download debug build</Button>
            </div>
        )
    }
};

export default DevDashboard;
