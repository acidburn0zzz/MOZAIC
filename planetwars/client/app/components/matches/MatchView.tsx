import * as React from 'react';
import Visualizer from '../visualizer/Visualizer';
import { Match, FinishedMatch, ErroredMatch } from './types';
import { div, h } from 'react-hyperscript-helpers';
import { parseLog } from '../../lib/match/log';

export interface MatchViewProps {
  match: Match;
}

export class MatchView extends React.Component<MatchViewProps> {

  public render() {
    const { match } = this.props;
    switch (match.status) {
      case 'finished': {
        const log = parseLog(match.players, this.props.match.logPath);
        return <Visualizer matchLog={log}/>;
      }
      case 'error': {
        return (
          <div>
            {match.error}
          </div>);
      }
      case 'playing': {
        return (
          <div>
            in progress
        </div>);
      }
    }
  }
}

export default MatchView;
