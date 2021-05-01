import React from 'react';
import { FormGroup, TextField, Button, createMuiTheme, MuiThemeProvider } from '@material-ui/core';
import { withStyles } from "@material-ui/core/styles";

import AppState from '../appstate';

const styles = {
  input: {
    color: "white"
  }
};
  

const palette = createMuiTheme({
    palette: {
        primary: { main: '#00e5ff', contrastText: 'white'},
        secondary: { main: '#2979ff', contrastText: 'white' },
    }
})

class LoginPage extends React.Component<{setAppState: Function, setLoggedIn: Function}, {password: string, badPassword: boolean}> {
    classes;

    constructor(props: any) {
        super(props)
        const {classes} = props;
        this.classes = classes;
        this.state = {
            password:'',
            badPassword: false,
        }
    }

    async handleClick(event: any) {
        event.preventDefault()

        if (!this.verifyPassword(this.state.password)) {
            this.setState({badPassword: true});
            return;
        }

        let response: Response = await fetch('/api/login', {
            method: 'post', 
            body: JSON.stringify(
                {password: this.state.password}
            ),
            headers: {
                'Content-Type': 'application/json'
              },
        });

        if(response.ok) {
            this.props.setLoggedIn(true)
            this.props.setAppState(AppState.DevDashboard)
        }
        else {
            this.setState({badPassword: true})
        }
    }

    verifyPassword(password: string): boolean {
        return btoa(password) == "bndUZFd5SzR1WG16VTlIa1Z3RURWaGhlM0VOQ2drZmE=";
    }

    handleChange(event: any, target: any) {
        let newState: any = {};
        newState[target] = event.target.value;
        this.setState(newState)
    }

    render() {
        let badPassword;
        if(this.state.badPassword) {
            badPassword = (<p>Couldn't log you in with the specified credentials</p>)
        }

        return (
            <MuiThemeProvider theme={palette}>
                <form onSubmit={(e) => this.handleClick(e)}>
                    <TextField
                        InputProps={{className: this.classes.input}}
                        variant="filled"
                        margin="normal"
                        required
                        fullWidth
                        name="password"
                        label="Password"
                        type="password"
                        id="password"
                        autoComplete="current-password"
                        value={this.state.password}
                        onChange={(e) => this.handleChange(e, "password")}
                    />
                    <Button id="submitLogin" color="primary" type="submit">Login</Button>
                </form>
                {badPassword}
            </MuiThemeProvider>
        )
    }
}

export default withStyles(styles)(LoginPage);
