import { ClientParams } from "./EventWire";
import { Event, EventType } from "../reactors/SimpleEventEmitter";
import { Handler } from "../reactors/RequestHandler";
import { Connection } from "./Connection";
import { TcpStreamHandler } from "./TcpStreamHandler";
import { encodeEvent } from "../reactors/utils";

export interface EventHandler {
    handleEvent: (any) => void;
    handleWireEvent: (WireEvent) => void;
}

export class Client {
    private tcpHandler: TcpStreamHandler;
    private connection: Connection;
    private params: ClientParams;

    constructor(params: ClientParams) {
        this.connection = new Connection();
        this.params = params;
        this.tcpHandler = new TcpStreamHandler(params, this.connection);
    }

    public on<T>(eventType: EventType<T>, handler: Handler<T>) {
        this.connection.on(eventType, handler);
    }

    public connect(message: Uint8Array) {
        this.tcpHandler.onConnect.one(() => {
            this.tcpHandler.openChannel(message, this.connection);
        })
        this.tcpHandler.connect();
    }

    public exit() {
        // TODO: clean termination
        this.connection.requestClose();
    }

    public send(event: Event) {
        const request = encodeEvent(event);
        this.connection.request(request);
    }

    public request<T>(event: Event, responseType: EventType<T>): Promise<T> {
        const request = encodeEvent(event);
        return this.connection.request(request, responseType);
    }
}