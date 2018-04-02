import * as protocol_root from './proto';
const proto = protocol_root.mozaic.protocol;
import * as net from 'net';
import * as stream from 'stream';
import * as Promise from 'bluebird';
import { EventEmitter } from 'events';
import { BufferWriter, BufferReader } from 'protobufjs/minimal';
import { read } from 'fs';



export class Address {
    readonly host: string;
    readonly port: number;

    public constructor(host: string, port: number) {
        this.host = host;
        this.port = port;
    }

    public connect() : Promise<net.Socket> {
        return new Promise((resolve, reject) => {
            let socket = net.connect({
                host: this.host,
                port: this.port,
            });

            socket.on('connect', () => resolve(socket));
            socket.on('error', e => reject(e));
        });
    }
}


enum ConnectionState {
    DISCONNECTED,
    CONNECTING,
    CONNECTED,
    CLOSED,
};

export class Connection extends EventEmitter {
    private token: Buffer;
    private state: ConnectionState;
    private socket?: net.Socket;

    private recvBuffer: Buffer;
    
    public constructor(token: Buffer) {
        super();
        this.token = token;
        this.recvBuffer = new Buffer(0);
        this.state = ConnectionState.DISCONNECTED;
    }

    public connect(socket: net.Socket) {
        this.socket = socket;
        // drop old receive buffer
        this.recvBuffer = new Buffer(0);

        // set callbacks
        // TODO: handle errors and such
        socket.on('data', (buf: Buffer) => this.readMessages(buf));

        // initiate handshake
        this.state = ConnectionState.CONNECTING;
        let request = proto.ConnectRequest.create({ token: this.token });
        this.writeMessage(proto.ConnectRequest.encode(request));
    }

    public send(data: Buffer) {
        let gameData = proto.GameData.create({ data });
        let msg = proto.ClientMessage.create({ gameData });
        this.writeMessage(proto.ClientMessage.encode(msg));
    }

    // write a write-op to the underlying socket.
    // it is illegal to call this when not connected.
    private writeMessage(write: BufferWriter) {
        let buf = write.ldelim().finish();
        this.socket!.write(buf);
    }

    private readMessage(buf: Buffer) {
        switch (this.state) {
            case ConnectionState.CONNECTING: {
                let response = proto.ConnectResponse.decode(buf);
                this.state = ConnectionState.CONNECTED;
                this.emit('connected');
                break;
            }
            case ConnectionState.CONNECTED: {
                let packet = proto.ClientMessage.decode(buf);
                if (packet.gameData) {
                    this.emit('message', packet.gameData.data);
                }
                break;
            }
            case ConnectionState.DISCONNECTED: {
                throw new Error(
                    "tried reading from a disconnected connection"
                );
            }
            case ConnectionState.CLOSED: {
                throw new Error(
                    "tried reading from a closed connection"
                );
            }
        }
    }

    private readMessages(bytes: Buffer) {
        // Buffer.concat returns a new buffer, so the bytes are copied over
        // from recvBuffer, so that the old value becomes garbage.
        this.recvBuffer = Buffer.concat([this.recvBuffer, bytes]);

        let pos = 0;
        let reader = new BufferReader(this.recvBuffer);

        while (pos < this.recvBuffer.length) {
            let end: number;
            try {
                // try reading segment length at pos
                reader.pos = pos;
                let len = reader.uint32();
                end = reader.pos + len;
            } catch(err) {
                if (err instanceof RangeError) {
                    // range errors are due to incomplete data
                    break;
                } else {
                    // other errors are not supposed to happen
                    throw(err);
                }
            }

            if (end > this.recvBuffer.length) {
                // not enough data
                break;
            }

            // reader.pos is now at the first byte after the segment length
            let bytes = this.recvBuffer.slice(reader.pos, end);
            this.readMessage(bytes);
            // advance position
            pos = end;
        }

        // set recvBuffer to a view of the current value to avoid realloc
        this.recvBuffer = this.recvBuffer.slice(pos);
    }
}