const React = require('react');
const h = require('react-hyperscript');
const {
  div,
  span,
  h1,
  button,
  i,
  input,
  p,
  svg
} = require('hyperscript-helpers')(h);

class ToggleButton extends React.Component {
  constructor(props) {
    super(props);
    this.state = {
      icon: this.props.icon1,
      title: this.props.title1
    };
    this.togged = this.props.toggled;
    this.toggle = this.toggle.bind(this);
  }

  render() {
    return button(`${this.props.selector}.control.control-button`, {
      'title': this.state.title,
      'type': 'button',
      'aria-hidden': 'true',
      'onClick': this.toggle
    }, [
      i(`.fa.fa-${this.state.icon}`)
    ]);
  }

  toggle() {
    if (this.toggled) {
      this.setState({
        icon: this.props.icon1,
        title: this.props.title1
      });
      this.props.callback1();
    } else {
      this.setState({
        icon: this.props.icon2,
        title: this.props.title2
      });
      this.props.callback2();
    }
    this.toggled = !this.toggled;
  }
}

class HideableComponent extends React.Component {
  render() {
    if (this.props.hide)
      return null;
    return this.props.render;
  }
}

class ControlButton extends React.Component {
  constructor(props) {
    super(props);
    this.click = this.click.bind(this);
  }

  render() {
    return button(`${this.props.selector}.control.control-button`, {
      'title': this.props.title,
      'type': 'button',
      'aria-hidden': 'true',
      'onClick': this.click
    }, [
      i(`.fa.fa-${this.props.icon}`)
    ]);
  }

  click() {
    this.props.callback();
  }
}



module.exports = {
  HideableComponent,
  ToggleButton,
  ControlButton
};
