import { EventWire, ClientParams } from "./EventWire";
import { Connected, Disconnected } from "../eventTypes";
import { Event } from "../reactors/SimpleEventEmitter";

export interface EventHandler {
    handleEvent: (any) => void;
    handleWireEvent: (WireEvent) => void;
}

export class Client {
    private eventWire: EventWire;
    public handler: EventHandler;
    private requestCounter: number;

    constructor(params: ClientParams, handler: EventHandler) {
        this.handler = handler;
        this.eventWire = new EventWire(params);
        this.requestCounter = 0;

        this.eventWire.onEvent.subscribe((event) => {
            this.handler.handleWireEvent(event);
        });

        this.eventWire.onConnect.subscribe((_) => {
            this.handler.handleEvent(Connected.create({}));
        });

        this.eventWire.onDisconnect.subscribe(() => {
            this.handler.handleEvent(Disconnected.create({}));
        });
    }

    public connect(message: Uint8Array) {
        this.eventWire.connect(message);
    }

    public exit() {
        this.eventWire.close();
    }

    public send(event: Event) {
        this.eventWire.send(event);
    }

    public nextRequestId(): number {
        this.requestCounter += 1;
        return this.requestCounter;
    }
}