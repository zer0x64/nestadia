import React, { CSSProperties } from 'react';
import './App.css';
import AppState from './appstate';
import DevDashboard from './devdashboard/devdashboard';
import Emulator from './emulator/emulator';
import EmulatorMode from './emulator/emulatorMode';
import LoginPage from './login/login';
import MainPage from './mainpage/mainpage';
import logo from "./logo-nestadia-background.png";

class App extends React.Component<{}, {state: AppState, isLoggedIn: boolean, mode: EmulatorMode}> {
  constructor(props: any) {
    super(props);

    this.state = {state: AppState.MainPage, isLoggedIn: false, mode: EmulatorMode.Normal}
  }

  setAppState(state: AppState) {
    this.setState({'state': state})
  }

  setEmulatorMode(mode: EmulatorMode) {
    this.setState({mode: mode})
  }

  setLoggedIn(isLoggedIn: boolean) {
    this.setState({isLoggedIn: isLoggedIn})
  }

  componentDidMount() {
    
  }

  render() {
    let content;
    if(this.state.state == AppState.MainPage) {
      content = (<MainPage setAppState={this.setAppState.bind(this)} setEmulatorMode={this.setEmulatorMode.bind(this)}></MainPage>)
    }
    else if(this.state.state == AppState.LoginPage) {
      content = (<LoginPage setAppState={this.setAppState.bind(this)} setLoggedIn={this.setLoggedIn.bind(this)}></LoginPage>)
    }
    else if(this.state.state == AppState.DevDashboard) {
      content = (<DevDashboard setAppState={this.setAppState.bind(this)} setEmulatorMode={this.setEmulatorMode.bind(this)}></DevDashboard>)
    }
    else {
      content = (<Emulator setAppState={this.setAppState.bind(this)} mode={this.state.mode}></Emulator>)
    }

    const styleLogo: CSSProperties = {
        marginBottom: "4vh",
    }

    return (
      <div className="App">
        <header className="App-header">
          <img style={styleLogo} src={logo} alt="logo"/>
          {content}
        </header>
      </div>
    );
  }
}

export default App;
