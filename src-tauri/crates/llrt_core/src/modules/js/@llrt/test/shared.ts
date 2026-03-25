export type SocketReqMsg
	= | ReadyReqMsg
		| ModuleReqMsg
		| EndReqMsg
		| StartReqMsg
		| CompletedReqMsg
		| ErrorReqMsg;

export interface ReadyReqMsg {
	type: 'ready'
	workerId: number
}

export interface ErrorReqMsg {
	type: 'error'
	error: any
	ended: number
	started: number
	workerId: number
}

export interface ModuleReqMsg {
	type: 'module'
	testCount: number
	skipCount: number
	onlyCount: number
}

export interface CompletedReqMsg {
	type: 'completed'
}

export interface EndReqMsg {
	type: 'end'
	ended: number
	started: number
	isSuite: boolean
}

export interface StartReqMsg {
	type: 'start'
	desc: string
	isSuite: boolean
	started: number
	timeout?: number
}

export interface SocketResponseMap {}

export type SocketRes<T extends SocketReqMsg> = T extends {
	type: keyof SocketResponseMap
}
	? SocketResponseMap[T['type']]
	: null;
