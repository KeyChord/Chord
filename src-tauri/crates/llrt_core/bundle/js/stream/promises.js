const yo = Object.create;
const bt = Object.defineProperty;
const wo = Object.getOwnPropertyDescriptor;
const go = Object.getOwnPropertyNames;
const So = Object.getPrototypeOf;
const Eo = Object.prototype.hasOwnProperty;
const o = (e, t) => bt(e, 'name', { value: t, configurable: !0 });
const N = (e =>
	typeof require < 'u'
		? require
		: typeof Proxy < 'u'
			? new Proxy(e, { get: (t, n) => (typeof require < 'u' ? require : t)[n] })
			: e)(function (e) {
	if (typeof require < 'u')
		return require.apply(this, arguments);
	throw new Error(`Dynamic require of "${e}" is not supported`);
});
const A = (e, t) => () => (t || e((t = { exports: {} }).exports, t), t.exports);
function Ro(e, t, n, r) {
	if ((t && typeof t == 'object') || typeof t == 'function') {
		for (const i of go(t)) {
			!Eo.call(e, i)
			&& i !== n
			&& bt(e, i, { get: () => t[i], enumerable: !(r = wo(t, i)) || r.enumerable });
		}
	}
	return e;
}
function Ao(e, t, n) {
	return (n = e != null ? yo(So(e)) : {}),
	Ro(t || !e || !e.__esModule ? bt(n, 'default', { value: e, enumerable: !0 }) : n, e);
}
const T = A((qs, mn) => {
	'use strict';
	const pt = class extends Error {
		static {
			o(this, 'AggregateError');
		}

		constructor(t) {
			if (!Array.isArray(t))
				throw new TypeError(`Expected input to be an Array, got ${typeof t}`);
			let n = '';
			for (let r = 0; r < t.length; r++) {
				n += `    ${t[r].stack}
`;
			}
			(super(n), (this.name = 'AggregateError'), (this.errors = t));
		}
	};
	mn.exports = {
		AggregateError: pt,
		ArrayIsArray(e) {
			return Array.isArray(e);
		},
		ArrayPrototypeIncludes(e, t) {
			return e.includes(t);
		},
		ArrayPrototypeIndexOf(e, t) {
			return e.indexOf(t);
		},
		ArrayPrototypeJoin(e, t) {
			return e.join(t);
		},
		ArrayPrototypeMap(e, t) {
			return e.map(t);
		},
		ArrayPrototypePop(e, t) {
			return e.pop(t);
		},
		ArrayPrototypePush(e, t) {
			return e.push(t);
		},
		ArrayPrototypeSlice(e, t, n) {
			return e.slice(t, n);
		},
		Error,
		FunctionPrototypeCall(e, t, ...n) {
			return e.call(t, ...n);
		},
		FunctionPrototypeSymbolHasInstance(e, t) {
			return Function.prototype[Symbol.hasInstance].call(e, t);
		},
		MathFloor: Math.floor,
		Number,
		NumberIsInteger: Number.isInteger,
		NumberIsNaN: Number.isNaN,
		NumberMAX_SAFE_INTEGER: Number.MAX_SAFE_INTEGER,
		NumberMIN_SAFE_INTEGER: Number.MIN_SAFE_INTEGER,
		NumberParseInt: Number.parseInt,
		ObjectDefineProperties(e, t) {
			return Object.defineProperties(e, t);
		},
		ObjectDefineProperty(e, t, n) {
			return Object.defineProperty(e, t, n);
		},
		ObjectGetOwnPropertyDescriptor(e, t) {
			return Object.getOwnPropertyDescriptor(e, t);
		},
		ObjectKeys(e) {
			return Object.keys(e);
		},
		ObjectSetPrototypeOf(e, t) {
			return Object.setPrototypeOf(e, t);
		},
		Promise,
		PromisePrototypeCatch(e, t) {
			return e.catch(t);
		},
		PromisePrototypeThen(e, t, n) {
			return e.then(t, n);
		},
		PromiseReject(e) {
			return Promise.reject(e);
		},
		PromiseResolve(e) {
			return Promise.resolve(e);
		},
		ReflectApply: Reflect.apply,
		RegExpPrototypeTest(e, t) {
			return e.test(t);
		},
		SafeSet: Set,
		String,
		StringPrototypeSlice(e, t, n) {
			return e.slice(t, n);
		},
		StringPrototypeToLowerCase(e) {
			return e.toLowerCase();
		},
		StringPrototypeToUpperCase(e) {
			return e.toUpperCase();
		},
		StringPrototypeTrim(e) {
			return e.trim();
		},
		Symbol,
		SymbolFor: Symbol.for,
		SymbolAsyncIterator: Symbol.asyncIterator,
		SymbolHasInstance: Symbol.hasInstance,
		SymbolIterator: Symbol.iterator,
		SymbolDispose: Symbol.dispose || Symbol('Symbol.dispose'),
		SymbolAsyncDispose: Symbol.asyncDispose || Symbol('Symbol.asyncDispose'),
		TypedArrayPrototypeSet(e, t, n) {
			return e.set(t, n);
		},
		Boolean,
		Uint8Array,
	};
});
const B = A((Ls, jn) => {
	'use strict';
	const { SymbolAsyncIterator: Tn, SymbolIterator: In, SymbolFor: ne } = T();
	const Mn = ne('nodejs.stream.destroyed');
	const kn = ne('nodejs.stream.errored');
	const _t = ne('nodejs.stream.readable');
	const yt = ne('nodejs.stream.writable');
	const Nn = ne('nodejs.stream.disturbed');
	const mo = ne('nodejs.webstream.isClosedPromise');
	const To = ne('nodejs.webstream.controllerErrorFunction');
	function Ce(e, t = !1) {
		let n;
		return !!(
			e
			&& typeof e.pipe == 'function'
			&& typeof e.on == 'function'
			&& (!t || (typeof e.pause == 'function' && typeof e.resume == 'function'))
			&& (!e._writableState
				|| ((n = e._readableState) === null || n === void 0 ? void 0 : n.readable) !== !1)
			&& (!e._writableState || e._readableState)
		);
	}
	o(Ce, 'isReadableNodeStream');
	function $e(e) {
		let t;
		return !!(
			e
			&& typeof e.write == 'function'
			&& typeof e.on == 'function'
			&& (!e._readableState
				|| ((t = e._writableState) === null || t === void 0 ? void 0 : t.writable) !== !1)
		);
	}
	o($e, 'isWritableNodeStream');
	function Io(e) {
		return !!(
			e
			&& typeof e.pipe == 'function'
			&& e._readableState
			&& typeof e.on == 'function'
			&& typeof e.write == 'function'
		);
	}
	o(Io, 'isDuplexNodeStream');
	function v(e) {
		return (
			e
			&& (e._readableState
				|| e._writableState
				|| (typeof e.write == 'function' && typeof e.on == 'function')
				|| (typeof e.pipe == 'function' && typeof e.on == 'function'))
		);
	}
	o(v, 'isNodeStream');
	function qn(e) {
		return !!(
			e
			&& !v(e)
			&& typeof e.pipeThrough == 'function'
			&& typeof e.getReader == 'function'
			&& typeof e.cancel == 'function'
		);
	}
	o(qn, 'isReadableStream');
	function Dn(e) {
		return !!(e && !v(e) && typeof e.getWriter == 'function' && typeof e.abort == 'function');
	}
	o(Dn, 'isWritableStream');
	function Ln(e) {
		return !!(e && !v(e) && typeof e.readable == 'object' && typeof e.writable == 'object');
	}
	o(Ln, 'isTransformStream');
	function Mo(e) {
		return qn(e) || Dn(e) || Ln(e);
	}
	o(Mo, 'isWebStream');
	function ko(e, t) {
		return e == null
			? !1
			: t === !0
				? typeof e[Tn] == 'function'
				: t === !1
					? typeof e[In] == 'function'
					: typeof e[Tn] == 'function' || typeof e[In] == 'function';
	}
	o(ko, 'isIterable');
	function je(e) {
		if (!v(e))
			return null;
		const t = e._writableState;
		const n = e._readableState;
		const r = t || n;
		return !!(e.destroyed || e[Mn] || (r != null && r.destroyed));
	}
	o(je, 'isDestroyed');
	function Pn(e) {
		if (!$e(e))
			return null;
		if (e.writableEnded === !0)
			return !0;
		const t = e._writableState;
		return t != null && t.errored ? !1 : typeof t?.ended != 'boolean' ? null : t.ended;
	}
	o(Pn, 'isWritableEnded');
	function No(e, t) {
		if (!$e(e))
			return null;
		if (e.writableFinished === !0)
			return !0;
		const n = e._writableState;
		return n != null && n.errored
			? !1
			: typeof n?.finished != 'boolean'
				? null
				: !!(n.finished || (t === !1 && n.ended === !0 && n.length === 0));
	}
	o(No, 'isWritableFinished');
	function qo(e) {
		if (!Ce(e))
			return null;
		if (e.readableEnded === !0)
			return !0;
		const t = e._readableState;
		return !t || t.errored ? !1 : typeof t?.ended != 'boolean' ? null : t.ended;
	}
	o(qo, 'isReadableEnded');
	function On(e, t) {
		if (!Ce(e))
			return null;
		const n = e._readableState;
		return n != null && n.errored
			? !1
			: typeof n?.endEmitted != 'boolean'
				? null
				: !!(n.endEmitted || (t === !1 && n.ended === !0 && n.length === 0));
	}
	o(On, 'isReadableFinished');
	function Wn(e) {
		return e && e[_t] != null
			? e[_t]
			: typeof e?.readable != 'boolean'
				? null
				: je(e)
					? !1
					: Ce(e) && e.readable && !On(e);
	}
	o(Wn, 'isReadable');
	function xn(e) {
		return e && e[yt] != null
			? e[yt]
			: typeof e?.writable != 'boolean'
				? null
				: je(e)
					? !1
					: $e(e) && e.writable && !Pn(e);
	}
	o(xn, 'isWritable');
	function Do(e, t) {
		return v(e)
			? je(e)
				? !0
				: !((t?.readable !== !1 && Wn(e)) || (t?.writable !== !1 && xn(e)))
			: null;
	}
	o(Do, 'isFinished');
	function Lo(e) {
		let t, n;
		return v(e)
			? e.writableErrored
				? e.writableErrored
				: (t = (n = e._writableState) === null || n === void 0 ? void 0 : n.errored) !== null
					&& t !== void 0
						? t
						: null
			: null;
	}
	o(Lo, 'isWritableErrored');
	function Po(e) {
		let t, n;
		return v(e)
			? e.readableErrored
				? e.readableErrored
				: (t = (n = e._readableState) === null || n === void 0 ? void 0 : n.errored) !== null
					&& t !== void 0
						? t
						: null
			: null;
	}
	o(Po, 'isReadableErrored');
	function Oo(e) {
		if (!v(e))
			return null;
		if (typeof e.closed == 'boolean')
			return e.closed;
		const t = e._writableState;
		const n = e._readableState;
		return typeof t?.closed == 'boolean' || typeof n?.closed == 'boolean'
			? t?.closed || n?.closed
			: typeof e._closed == 'boolean' && Cn(e)
				? e._closed
				: null;
	}
	o(Oo, 'isClosed');
	function Cn(e) {
		return (
			typeof e._closed == 'boolean'
			&& typeof e._defaultKeepAlive == 'boolean'
			&& typeof e._removedConnection == 'boolean'
			&& typeof e._removedContLen == 'boolean'
		);
	}
	o(Cn, 'isOutgoingMessage');
	function $n(e) {
		return typeof e._sent100 == 'boolean' && Cn(e);
	}
	o($n, 'isServerResponse');
	function Wo(e) {
		let t;
		return (
			typeof e._consuming == 'boolean'
			&& typeof e._dumped == 'boolean'
			&& ((t = e.req) === null || t === void 0 ? void 0 : t.upgradeOrConnect) === void 0
		);
	}
	o(Wo, 'isServerRequest');
	function xo(e) {
		if (!v(e))
			return null;
		const t = e._writableState;
		const n = e._readableState;
		const r = t || n;
		return (!r && $n(e)) || !!(r && r.autoDestroy && r.emitClose && r.closed === !1);
	}
	o(xo, 'willEmitClose');
	function Co(e) {
		let t;
		return !!(
			e && ((t = e[Nn]) !== null && t !== void 0 ? t : e.readableDidRead || e.readableAborted)
		);
	}
	o(Co, 'isDisturbed');
	function $o(e) {
		let t, n, r, i, l, a, u, s, f, d;
		return !!(
			e
			&& ((t
				= (n
					= (r
						= (i
							= (l = (a = e[kn]) !== null && a !== void 0 ? a : e.readableErrored) !== null
								&& l !== void 0
								? l
								: e.writableErrored) !== null && i !== void 0
							? i
							: (u = e._readableState) === null || u === void 0
									? void 0
									: u.errorEmitted) !== null && r !== void 0
						? r
						: (s = e._writableState) === null || s === void 0
								? void 0
								: s.errorEmitted) !== null && n !== void 0
					? n
					: (f = e._readableState) === null || f === void 0
							? void 0
							: f.errored) !== null && t !== void 0
				? t
				: !((d = e._writableState) === null || d === void 0) && d.errored)
		);
	}
	o($o, 'isErrored');
	jn.exports = {
		isDestroyed: je,
		kIsDestroyed: Mn,
		isDisturbed: Co,
		kIsDisturbed: Nn,
		isErrored: $o,
		kIsErrored: kn,
		isReadable: Wn,
		kIsReadable: _t,
		kIsClosedPromise: mo,
		kControllerErrorFunction: To,
		kIsWritable: yt,
		isClosed: Oo,
		isDuplexNodeStream: Io,
		isFinished: Do,
		isIterable: ko,
		isReadableNodeStream: Ce,
		isReadableStream: qn,
		isReadableEnded: qo,
		isReadableFinished: On,
		isReadableErrored: Po,
		isNodeStream: v,
		isWebStream: Mo,
		isWritable: xn,
		isWritableNodeStream: $e,
		isWritableStream: Dn,
		isWritableEnded: Pn,
		isWritableFinished: No,
		isWritableErrored: Lo,
		isServerRequest: Wo,
		isServerResponse: $n,
		willEmitClose: xo,
		isTransformStream: Ln,
	};
});
const wt = A((Os, Fn) => {
	'use strict';
	Fn.exports = {
		format(e, ...t) {
			return e.replace(/%([sdifj])/g, (...[n, r]) => {
				const i = t.shift();
				return r === 'f'
					? i.toFixed(6)
					: r === 'j'
						? JSON.stringify(i)
						: r === 's' && typeof i == 'object'
							? `${i.constructor !== Object ? i.constructor.name : ''} {}`.trim()
							: i.toString();
			});
		},
		inspect(e) {
			switch (typeof e) {
				case 'string':
					if (e.includes('\'')) {
						if (e.includes('"')) {
							if (!e.includes('`') && !e.includes('${'))
								return `\`${e}\``;
						}
						else {
							return `"${e}"`;
						}
					}
					return `'${e}'`;
				case 'number':
					return isNaN(e) ? 'NaN' : Object.is(e, -0) ? String(e) : e;
				case 'bigint':
					return `${String(e)}n`;
				case 'boolean':
				case 'undefined':
					return String(e);
				case 'object':
					return '{}';
			}
		},
	};
});
const L = A((Ws, Un) => {
	'use strict';
	const { format: jo, inspect: Fe } = wt();
	const { AggregateError: Fo } = T();
	const vo = globalThis.AggregateError || Fo;
	const Bo = Symbol('kIsNodeError');
	const Uo = [
		'string',
		'function',
		'number',
		'object',
		'Function',
		'Object',
		'boolean',
		'bigint',
		'symbol',
	];
	const Ho = /^([A-Z][a-z0-9]*)+$/;
	const Go = '__node_internal_';
	const ve = {};
	function re(e, t) {
		if (!e)
			throw new ve.ERR_INTERNAL_ASSERTION(t);
	}
	o(re, 'assert');
	function vn(e) {
		let t = '';
		let n = e.length;
		const r = e[0] === '-' ? 1 : 0;
		for (; n >= r + 4; n -= 3) t = `_${e.slice(n - 3, n)}${t}`;
		return `${e.slice(0, n)}${t}`;
	}
	o(vn, 'addNumericalSeparator');
	function Vo(e, t, n) {
		if (typeof t == 'function') {
			return (
				re(
					t.length <= n.length,
					`Code: ${e}; The provided arguments length (${n.length}) does not match the required ones (${t.length}).`,
				),
				t(...n)
			);
		}
		const r = (t.match(/%[dfijoOs]/g) || []).length;
		return (
			re(
				r === n.length,
				`Code: ${e}; The provided arguments length (${n.length}) does not match the required ones (${r}).`,
			),
			n.length === 0 ? t : jo(t, ...n)
		);
	}
	o(Vo, 'getMessage');
	function D(e, t, n) {
		n || (n = Error);
		class r extends n {
			static {
				o(this, 'NodeError');
			}

			constructor(...l) {
				super(Vo(e, t, l));
			}

			toString() {
				return `${this.name} [${e}]: ${this.message}`;
			}
		}
		(Object.defineProperties(r.prototype, {
			name: { value: n.name, writable: !0, enumerable: !1, configurable: !0 },
			toString: {
				value() {
					return `${this.name} [${e}]: ${this.message}`;
				},
				writable: !0,
				enumerable: !1,
				configurable: !0,
			},
		}),
		(r.prototype.code = e),
		(r.prototype[Bo] = !0),
		(ve[e] = r));
	}
	o(D, 'E');
	function Bn(e) {
		const t = Go + e.name;
		return (Object.defineProperty(e, 'name', { value: t }), e);
	}
	o(Bn, 'hideStackFrames');
	function Yo(e, t) {
		if (e && t && e !== t) {
			if (Array.isArray(t.errors))
				return (t.errors.push(e), t);
			const n = new vo([t, e], t.message);
			return ((n.code = t.code), n);
		}
		return e || t;
	}
	o(Yo, 'aggregateTwoErrors');
	const gt = class extends Error {
		static {
			o(this, 'AbortError');
		}

		constructor(t = 'The operation was aborted', n = void 0) {
			if (n !== void 0 && typeof n != 'object')
				throw new ve.ERR_INVALID_ARG_TYPE('options', 'Object', n);
			(super(t, n), (this.code = 'ABORT_ERR'), (this.name = 'AbortError'));
		}
	};
	D('ERR_ASSERTION', '%s', Error);
	D(
		'ERR_INVALID_ARG_TYPE',
		(e, t, n) => {
			(re(typeof e == 'string', '\'name\' must be a string'), Array.isArray(t) || (t = [t]));
			let r = 'The ';
			(e.endsWith(' argument')
				? (r += `${e} `)
				: (r += `"${e}" ${e.includes('.') ? 'property' : 'argument'} `),
			(r += 'must be '));
			const i = [];
			const l = [];
			const a = [];
			for (const s of t) {
				(re(typeof s == 'string', 'All expected entries have to be of type string'),
				Uo.includes(s)
					? i.push(s.toLowerCase())
					: Ho.test(s)
						? l.push(s)
						: (re(s !== 'object', 'The value "object" should be written as "Object"'),
							a.push(s)));
			}
			if (l.length > 0) {
				const s = i.indexOf('object');
				s !== -1 && (i.splice(i, s, 1), l.push('Object'));
			}
			if (i.length > 0) {
				switch (i.length) {
					case 1:
						r += `of type ${i[0]}`;
						break;
					case 2:
						r += `one of type ${i[0]} or ${i[1]}`;
						break;
					default: {
						const s = i.pop();
						r += `one of type ${i.join(', ')}, or ${s}`;
					}
				}
				(l.length > 0 || a.length > 0) && (r += ' or ');
			}
			if (l.length > 0) {
				switch (l.length) {
					case 1:
						r += `an instance of ${l[0]}`;
						break;
					case 2:
						r += `an instance of ${l[0]} or ${l[1]}`;
						break;
					default: {
						const s = l.pop();
						r += `an instance of ${l.join(', ')}, or ${s}`;
					}
				}
				a.length > 0 && (r += ' or ');
			}
			switch (a.length) {
				case 0:
					break;
				case 1:
					(a[0].toLowerCase() !== a[0] && (r += 'an '), (r += `${a[0]}`));
					break;
				case 2:
					r += `one of ${a[0]} or ${a[1]}`;
					break;
				default: {
					const s = a.pop();
					r += `one of ${a.join(', ')}, or ${s}`;
				}
			}
			if (n == null) {
				r += `. Received ${n}`;
			}
			else if (typeof n == 'function' && n.name) {
				r += `. Received function ${n.name}`;
			}
			else if (typeof n == 'object') {
				let u;
				if ((u = n.constructor) !== null && u !== void 0 && u.name) {
					r += `. Received an instance of ${n.constructor.name}`;
				}
				else {
					const s = Fe(n, { depth: -1 });
					r += `. Received ${s}`;
				}
			}
			else {
				let s = Fe(n, { colors: !1 });
				(s.length > 25 && (s = `${s.slice(0, 25)}...`),
				(r += `. Received type ${typeof n} (${s})`));
			}
			return r;
		},
		TypeError,
	);
	D(
		'ERR_INVALID_ARG_VALUE',
		(e, t, n = 'is invalid') => {
			let r = Fe(t);
			return (
				r.length > 128 && (r = `${r.slice(0, 128)}...`),
				`The ${e.includes('.') ? 'property' : 'argument'} '${e}' ${n}. Received ${r}`
			);
		},
		TypeError,
	);
	D(
		'ERR_INVALID_RETURN_VALUE',
		(e, t, n) => {
			let r;
			const i
				= n != null && (r = n.constructor) !== null && r !== void 0 && r.name
					? `instance of ${n.constructor.name}`
					: `type ${typeof n}`;
			return `Expected ${e} to be returned from the "${t}" function but got ${i}.`;
		},
		TypeError,
	);
	D(
		'ERR_MISSING_ARGS',
		(...e) => {
			re(e.length > 0, 'At least one arg needs to be specified');
			let t;
			const n = e.length;
			switch (((e = (Array.isArray(e) ? e : [e]).map(r => `"${r}"`).join(' or ')), n)) {
				case 1:
					t += `The ${e[0]} argument`;
					break;
				case 2:
					t += `The ${e[0]} and ${e[1]} arguments`;
					break;
				default:
					{
						const r = e.pop();
						t += `The ${e.join(', ')}, and ${r} arguments`;
					}
					break;
			}
			return `${t} must be specified`;
		},
		TypeError,
	);
	D(
		'ERR_OUT_OF_RANGE',
		(e, t, n) => {
			re(t, 'Missing "range" argument');
			let r;
			if (Number.isInteger(n) && Math.abs(n) > 2 ** 32) {
				r = vn(String(n));
			}
			else if (typeof n == 'bigint') {
				r = String(n);
				const i = BigInt(2) ** BigInt(32);
				((n > i || n < -i) && (r = vn(r)), (r += 'n'));
			}
			else {
				r = Fe(n);
			}
			return `The value of "${e}" is out of range. It must be ${t}. Received ${r}`;
		},
		RangeError,
	);
	D('ERR_MULTIPLE_CALLBACK', 'Callback called multiple times', Error);
	D('ERR_METHOD_NOT_IMPLEMENTED', 'The %s method is not implemented', Error);
	D('ERR_STREAM_ALREADY_FINISHED', 'Cannot call %s after a stream was finished', Error);
	D('ERR_STREAM_CANNOT_PIPE', 'Cannot pipe, not readable', Error);
	D('ERR_STREAM_DESTROYED', 'Cannot call %s after a stream was destroyed', Error);
	D('ERR_STREAM_NULL_VALUES', 'May not write null values to stream', TypeError);
	D('ERR_STREAM_PREMATURE_CLOSE', 'Premature close', Error);
	D('ERR_STREAM_PUSH_AFTER_EOF', 'stream.push() after EOF', Error);
	D('ERR_STREAM_UNSHIFT_AFTER_END_EVENT', 'stream.unshift() after end event', Error);
	D('ERR_STREAM_WRITE_AFTER_END', 'write after end', Error);
	D('ERR_UNKNOWN_ENCODING', 'Unknown encoding: %s', TypeError);
	Un.exports = { AbortError: gt, aggregateTwoErrors: Bn(Yo), hideStackFrames: Bn, codes: ve };
});
const he = A((Cs, Be) => {
	'use strict';
	const { AbortController: Hn, AbortSignal: Ko }
		= typeof self < 'u' ? self : typeof window < 'u' ? window : void 0;
	Be.exports = Hn;
	Be.exports.AbortSignal = Ko;
	Be.exports.default = Hn;
});
const O = A(($s, Et) => {
	'use strict';
	const zo = N('buffer');
	const { format: Xo, inspect: Jo } = wt();
	const {
		codes: { ERR_INVALID_ARG_TYPE: St },
	} = L();
	const { kResistStopPropagation: Qo, AggregateError: Zo, SymbolDispose: el } = T();
	const tl = globalThis.AbortSignal || he().AbortSignal;
	const nl = globalThis.AbortController || he().AbortController;
	const rl = Object.getPrototypeOf(async () => {}).constructor;
	const Gn = globalThis.Blob || zo.Blob;
	const il = o(
		typeof Gn < 'u'
			? (t) => {
					return t instanceof Gn;
				}
			: (t) => {
					return !1;
				},
		'isBlob',
	);
	const Vn = o((e, t) => {
		if (e !== void 0 && (e === null || typeof e != 'object' || !('aborted' in e)))
			throw new St(t, 'AbortSignal', e);
	}, 'validateAbortSignal');
	const ol = o((e, t) => {
		if (typeof e != 'function')
			throw new St(t, 'Function', e);
	}, 'validateFunction');
	Et.exports = {
		AggregateError: Zo,
		kEmptyObject: Object.freeze({}),
		once(e) {
			let t = !1;
			return function (...n) {
				t || ((t = !0), e.apply(this, n));
			};
		},
		createDeferredPromise: o(() => {
			let e, t;
			return {
				promise: new Promise((r, i) => {
					((e = r), (t = i));
				}),
				resolve: e,
				reject: t,
			};
		}, 'createDeferredPromise'),
		promisify(e) {
			return new Promise((t, n) => {
				e((r, ...i) => (r ? n(r) : t(...i)));
			});
		},
		debuglog() {
			return function () {};
		},
		format: Xo,
		inspect: Jo,
		types: {
			isAsyncFunction(e) {
				return e instanceof rl;
			},
			isArrayBufferView(e) {
				return ArrayBuffer.isView(e);
			},
		},
		isBlob: il,
		deprecate(e, t) {
			return e;
		},
		addAbortListener:
      N('events').addAbortListener
      || o((t, n) => {
      	if (t === void 0)
      		throw new St('signal', 'AbortSignal', t);
      	(Vn(t, 'signal'), ol(n, 'listener'));
      	let r;
      	return (
      		t.aborted
      			? queueMicrotask(() => n())
      			: (t.addEventListener('abort', n, { __proto__: null, once: !0, [Qo]: !0 }),
      				(r = o(() => {
      					t.removeEventListener('abort', n);
      				}, 'removeEventListener'))),
      		{
      			__proto__: null,
      			[el]() {
      				let i;
      				(i = r) === null || i === void 0 || i();
      			},
      		}
      	);
      }, 'addAbortListener'),
		AbortSignalAny:
      tl.any
      || o((t) => {
      	if (t.length === 1)
      		return t[0];
      	const n = new nl();
      	const r = o(() => n.abort(), 'abort');
      	return (
      		t.forEach((i) => {
      			(Vn(i, 'signals'), i.addEventListener('abort', r, { once: !0 }));
      		}),
      		n.signal.addEventListener(
      			'abort',
      			() => {
      				t.forEach(i => i.removeEventListener('abort', r));
      			},
      			{ once: !0 },
      		),
      		n.signal
      	);
      }, 'AbortSignalAny'),
	};
	Et.exports.promisify.custom = Symbol.for('nodejs.util.promisify.custom');
});
const pe = A((Fs, nr) => {
	'use strict';
	const {
		ArrayIsArray: At,
		ArrayPrototypeIncludes: Xn,
		ArrayPrototypeJoin: Jn,
		ArrayPrototypeMap: ll,
		NumberIsInteger: mt,
		NumberIsNaN: al,
		NumberMAX_SAFE_INTEGER: fl,
		NumberMIN_SAFE_INTEGER: ul,
		NumberParseInt: sl,
		ObjectPrototypeHasOwnProperty: dl,
		RegExpPrototypeExec: Qn,
		String: cl,
		StringPrototypeToUpperCase: hl,
		StringPrototypeTrim: bl,
	} = T();
	const {
		hideStackFrames: C,
		codes: {
			ERR_SOCKET_BAD_PORT: pl,
			ERR_INVALID_ARG_TYPE: P,
			ERR_INVALID_ARG_VALUE: be,
			ERR_OUT_OF_RANGE: ie,
			ERR_UNKNOWN_SIGNAL: Yn,
		},
	} = L();
	const { normalizeEncoding: _l } = O();
	const { isAsyncFunction: yl, isArrayBufferView: wl } = O().types;
	const Kn = {};
	function gl(e) {
		return e === (e | 0);
	}
	o(gl, 'isInt32');
	function Sl(e) {
		return e === e >>> 0;
	}
	o(Sl, 'isUint32');
	const El = /^[0-7]+$/;
	const Rl = 'must be a 32-bit unsigned integer or an octal string';
	function Al(e, t, n) {
		if ((typeof e > 'u' && (e = n), typeof e == 'string')) {
			if (Qn(El, e) === null)
				throw new be(t, e, Rl);
			e = sl(e, 8);
		}
		return (Zn(e, t), e);
	}
	o(Al, 'parseFileMode');
	const ml = C((e, t, n = ul, r = fl) => {
		if (typeof e != 'number')
			throw new P(t, 'number', e);
		if (!mt(e))
			throw new ie(t, 'an integer', e);
		if (e < n || e > r)
			throw new ie(t, `>= ${n} && <= ${r}`, e);
	});
	const Tl = C((e, t, n = -2147483648, r = 2147483647) => {
		if (typeof e != 'number')
			throw new P(t, 'number', e);
		if (!mt(e))
			throw new ie(t, 'an integer', e);
		if (e < n || e > r)
			throw new ie(t, `>= ${n} && <= ${r}`, e);
	});
	var Zn = C((e, t, n = !1) => {
		if (typeof e != 'number')
			throw new P(t, 'number', e);
		if (!mt(e))
			throw new ie(t, 'an integer', e);
		const r = n ? 1 : 0;
		const i = 4294967295;
		if (e < r || e > i)
			throw new ie(t, `>= ${r} && <= ${i}`, e);
	});
	function Tt(e, t) {
		if (typeof e != 'string')
			throw new P(t, 'string', e);
	}
	o(Tt, 'validateString');
	function Il(e, t, n = void 0, r) {
		if (typeof e != 'number')
			throw new P(t, 'number', e);
		if ((n != null && e < n) || (r != null && e > r) || ((n != null || r != null) && al(e))) {
			throw new ie(
				t,
				`${n != null ? `>= ${n}` : ''}${n != null && r != null ? ' && ' : ''}${r != null ? `<= ${r}` : ''}`,
				e,
			);
		}
	}
	o(Il, 'validateNumber');
	const Ml = C((e, t, n) => {
		if (!Xn(n, e)) {
			const i
				= `must be one of: ${
					Jn(
						ll(n, l => (typeof l == 'string' ? `'${l}'` : cl(l))),
						', ',
					)}`;
			throw new be(t, e, i);
		}
	});
	function er(e, t) {
		if (typeof e != 'boolean')
			throw new P(t, 'boolean', e);
	}
	o(er, 'validateBoolean');
	function Rt(e, t, n) {
		return e == null || !dl(e, t) ? n : e[t];
	}
	o(Rt, 'getOwnPropertyValueOrDefault');
	const kl = C((e, t, n = null) => {
		const r = Rt(n, 'allowArray', !1);
		const i = Rt(n, 'allowFunction', !1);
		if (
			(!Rt(n, 'nullable', !1) && e === null)
			|| (!r && At(e))
			|| (typeof e != 'object' && (!i || typeof e != 'function'))
		) {
			throw new P(t, 'Object', e);
		}
	});
	const Nl = C((e, t) => {
		if (e != null && typeof e != 'object' && typeof e != 'function')
			throw new P(t, 'a dictionary', e);
	});
	const Ue = C((e, t, n = 0) => {
		if (!At(e))
			throw new P(t, 'Array', e);
		if (e.length < n) {
			const r = `must be longer than ${n}`;
			throw new be(t, e, r);
		}
	});
	function ql(e, t) {
		Ue(e, t);
		for (let n = 0; n < e.length; n++) Tt(e[n], `${t}[${n}]`);
	}
	o(ql, 'validateStringArray');
	function Dl(e, t) {
		Ue(e, t);
		for (let n = 0; n < e.length; n++) er(e[n], `${t}[${n}]`);
	}
	o(Dl, 'validateBooleanArray');
	function Ll(e, t) {
		Ue(e, t);
		for (let n = 0; n < e.length; n++) {
			const r = e[n];
			const i = `${t}[${n}]`;
			if (r == null)
				throw new P(i, 'AbortSignal', r);
			tr(r, i);
		}
	}
	o(Ll, 'validateAbortSignalArray');
	function Pl(e, t = 'signal') {
		if ((Tt(e, t), Kn[e] === void 0)) {
			throw Kn[hl(e)] !== void 0
				? new Yn(`${e} (signals must use all capital letters)`)
				: new Yn(e);
		}
	}
	o(Pl, 'validateSignalName');
	const Ol = C((e, t = 'buffer') => {
		if (!wl(e))
			throw new P(t, ['Buffer', 'TypedArray', 'DataView'], e);
	});
	function Wl(e, t) {
		const n = _l(t);
		const r = e.length;
		if (n === 'hex' && r % 2 !== 0)
			throw new be('encoding', t, `is invalid for data of length ${r}`);
	}
	o(Wl, 'validateEncoding');
	function xl(e, t = 'Port', n = !0) {
		if (
			(typeof e != 'number' && typeof e != 'string')
			|| (typeof e == 'string' && bl(e).length === 0)
			|| +e !== +e >>> 0
			|| e > 65535
			|| (e === 0 && !n)
		) {
			throw new pl(t, e, n);
		}
		return e | 0;
	}
	o(xl, 'validatePort');
	var tr = C((e, t) => {
		if (e !== void 0 && (e === null || typeof e != 'object' || !('aborted' in e)))
			throw new P(t, 'AbortSignal', e);
	});
	const Cl = C((e, t) => {
		if (typeof e != 'function')
			throw new P(t, 'Function', e);
	});
	const $l = C((e, t) => {
		if (typeof e != 'function' || yl(e))
			throw new P(t, 'Function', e);
	});
	const jl = C((e, t) => {
		if (e !== void 0)
			throw new P(t, 'undefined', e);
	});
	function Fl(e, t, n) {
		if (!Xn(n, e))
			throw new P(t, `('${Jn(n, '|')}')`, e);
	}
	o(Fl, 'validateUnion');
	const vl = /^<[^>]*>(?:\s*;\s*[^;"\s]+(?:=(")?[^;"\s]*\1)?)*$/;
	function zn(e, t) {
		if (typeof e > 'u' || !Qn(vl, e)) {
			throw new be(
				t,
				e,
				'must be an array or string of format "</styles.css>; rel=preload; as=style"',
			);
		}
	}
	o(zn, 'validateLinkHeaderFormat');
	function Bl(e) {
		if (typeof e == 'string')
			return (zn(e, 'hints'), e);
		if (At(e)) {
			const t = e.length;
			let n = '';
			if (t === 0)
				return n;
			for (let r = 0; r < t; r++) {
				const i = e[r];
				(zn(i, 'hints'), (n += i), r !== t - 1 && (n += ', '));
			}
			return n;
		}
		throw new be(
			'hints',
			e,
			'must be an array or string of format "</styles.css>; rel=preload; as=style"',
		);
	}
	o(Bl, 'validateLinkHeaderValue');
	nr.exports = {
		isInt32: gl,
		isUint32: Sl,
		parseFileMode: Al,
		validateArray: Ue,
		validateStringArray: ql,
		validateBooleanArray: Dl,
		validateAbortSignalArray: Ll,
		validateBoolean: er,
		validateBuffer: Ol,
		validateDictionary: Nl,
		validateEncoding: Wl,
		validateFunction: Cl,
		validateInt32: Tl,
		validateInteger: ml,
		validateNumber: Il,
		validateObject: kl,
		validateOneOf: Ml,
		validatePlainFunction: $l,
		validatePort: xl,
		validateSignalName: Pl,
		validateString: Tt,
		validateUint32: Zn,
		validateUndefined: jl,
		validateUnion: Fl,
		validateAbortSignal: tr,
		validateLinkHeaderValue: Bl,
	};
});
const Y = A((Bs, qt) => {
	'use strict';
	const Q = N('process');
	const { AbortError: dr, codes: Ul } = L();
	const { ERR_INVALID_ARG_TYPE: Hl, ERR_STREAM_PREMATURE_CLOSE: rr } = Ul;
	const { kEmptyObject: Mt, once: kt } = O();
	const {
		validateAbortSignal: Gl,
		validateFunction: Vl,
		validateObject: Yl,
		validateBoolean: Kl,
	} = pe();
	const { Promise: zl, PromisePrototypeThen: Xl, SymbolDispose: cr } = T();
	const {
		isClosed: Jl,
		isReadable: ir,
		isReadableNodeStream: It,
		isReadableStream: Ql,
		isReadableFinished: or,
		isReadableErrored: lr,
		isWritable: ar,
		isWritableNodeStream: fr,
		isWritableStream: Zl,
		isWritableFinished: ur,
		isWritableErrored: sr,
		isNodeStream: ea,
		willEmitClose: ta,
		kIsClosedPromise: na,
	} = B();
	let _e;
	function ra(e) {
		return e.setHeader && typeof e.abort == 'function';
	}
	o(ra, 'isRequest');
	const Nt = o(() => {}, 'nop');
	function hr(e, t, n) {
		let r, i;
		if (
			(arguments.length === 2 ? ((n = t), (t = Mt)) : t == null ? (t = Mt) : Yl(t, 'options'),
			Vl(n, 'callback'),
			Gl(t.signal, 'options.signal'),
			(n = kt(n)),
			Ql(e) || Zl(e))) {
			return ia(e, t, n);
		}
		if (!ea(e))
			throw new Hl('stream', ['ReadableStream', 'WritableStream', 'Stream'], e);
		const l = (r = t.readable) !== null && r !== void 0 ? r : It(e);
		const a = (i = t.writable) !== null && i !== void 0 ? i : fr(e);
		const u = e._writableState;
		const s = e._readableState;
		const f = o(() => {
			e.writable || p();
		}, 'onlegacyfinish');
		let d = ta(e) && It(e) === l && fr(e) === a;
		let c = ur(e, !1);
		let p = o(() => {
			((c = !0), e.destroyed && (d = !1), !(d && (!e.readable || l)) && (!l || h) && n.call(e));
		}, 'onfinish');
		let h = or(e, !1);
		const S = o(() => {
			((h = !0), e.destroyed && (d = !1), !(d && (!e.writable || a)) && (!a || c) && n.call(e));
		}, 'onend');
		const b = o((M) => {
			n.call(e, M);
		}, 'onerror');
		let E = Jl(e);
		const y = o(() => {
			E = !0;
			const M = sr(e) || lr(e);
			if (M && typeof M != 'boolean')
				return n.call(e, M);
			if (l && !h && It(e, !0) && !or(e, !1))
				return n.call(e, new rr());
			if (a && !c && !ur(e, !1))
				return n.call(e, new rr());
			n.call(e);
		}, 'onclose');
		const R = o(() => {
			E = !0;
			const M = sr(e) || lr(e);
			if (M && typeof M != 'boolean')
				return n.call(e, M);
			n.call(e);
		}, 'onclosed');
		const k = o(() => {
			e.req.on('finish', p);
		}, 'onrequest');
		(ra(e)
			? (e.on('complete', p), d || e.on('abort', y), e.req ? k() : e.on('request', k))
			: a && !u && (e.on('end', f), e.on('close', f)),
		!d && typeof e.aborted == 'boolean' && e.on('aborted', y),
		e.on('end', S),
		e.on('finish', p),
		t.error !== !1 && e.on('error', b),
		e.on('close', y),
		E
			? Q.nextTick(y)
			: (u != null && u.errorEmitted) || (s != null && s.errorEmitted)
					? d || Q.nextTick(R)
					: ((!l && (!d || ir(e)) && (c || ar(e) === !1))
						|| (!a && (!d || ar(e)) && (h || ir(e) === !1))
						|| (s && e.req && e.aborted))
					&& Q.nextTick(R));
		const g = o(() => {
			((n = Nt),
			e.removeListener('aborted', y),
			e.removeListener('complete', p),
			e.removeListener('abort', y),
			e.removeListener('request', k),
			e.req && e.req.removeListener('finish', p),
			e.removeListener('end', f),
			e.removeListener('close', f),
			e.removeListener('finish', p),
			e.removeListener('end', S),
			e.removeListener('error', b),
			e.removeListener('close', y));
		}, 'cleanup');
		if (t.signal && !E) {
			const M = o(() => {
				const te = n;
				(g(), te.call(e, new dr(void 0, { cause: t.signal.reason })));
			}, 'abort');
			if (t.signal.aborted) {
				Q.nextTick(M);
			}
			else {
				_e = _e || O().addAbortListener;
				const te = _e(t.signal, M);
				const x = n;
				n = kt((...ce) => {
					(te[cr](), x.apply(e, ce));
				});
			}
		}
		return g;
	}
	o(hr, 'eos');
	function ia(e, t, n) {
		let r = !1;
		let i = Nt;
		if (t.signal) {
			if (
				((i = o(() => {
					((r = !0), n.call(e, new dr(void 0, { cause: t.signal.reason })));
				}, 'abort')),
				t.signal.aborted)) {
				Q.nextTick(i);
			}
			else {
				_e = _e || O().addAbortListener;
				const a = _e(t.signal, i);
				const u = n;
				n = kt((...s) => {
					(a[cr](), u.apply(e, s));
				});
			}
		}
		const l = o((...a) => {
			r || Q.nextTick(() => n.apply(e, a));
		}, 'resolverFn');
		return (Xl(e[na].promise, l, l), Nt);
	}
	o(ia, 'eosWeb');
	function oa(e, t) {
		let n;
		let r = !1;
		return (
			t === null && (t = Mt),
			(n = t) !== null && n !== void 0 && n.cleanup && (Kl(t.cleanup, 'cleanup'), (r = t.cleanup)),
			new zl((i, l) => {
				const a = hr(e, t, (u) => {
					(r && a(), u ? l(u) : i());
				});
			})
		);
	}
	o(oa, 'finished');
	qt.exports = hr;
	qt.exports.finished = oa;
});
const oe = A((Hs, Er) => {
	'use strict';
	const U = N('process');
	const {
		aggregateTwoErrors: la,
		codes: { ERR_MULTIPLE_CALLBACK: aa },
		AbortError: fa,
	} = L();
	const { Symbol: _r } = T();
	const { kIsDestroyed: ua, isDestroyed: sa, isFinished: da, isServerRequest: ca } = B();
	const yr = _r('kDestroy');
	const Dt = _r('kConstruct');
	function wr(e, t, n) {
		e && (e.stack, t && !t.errored && (t.errored = e), n && !n.errored && (n.errored = e));
	}
	o(wr, 'checkError');
	function ha(e, t) {
		const n = this._readableState;
		const r = this._writableState;
		const i = r || n;
		return (r != null && r.destroyed) || (n != null && n.destroyed)
			? (typeof t == 'function' && t(), this)
			: (wr(e, r, n),
				r && (r.destroyed = !0),
				n && (n.destroyed = !0),
				i.constructed
					? br(this, e, t)
					: this.once(yr, function (l) {
							br(this, la(l, e), t);
						}),
				this);
	}
	o(ha, 'destroy');
	function br(e, t, n) {
		let r = !1;
		function i(l) {
			if (r)
				return;
			r = !0;
			const a = e._readableState;
			const u = e._writableState;
			(wr(l, u, a),
			u && (u.closed = !0),
			a && (a.closed = !0),
			typeof n == 'function' && n(l),
			l ? U.nextTick(ba, e, l) : U.nextTick(gr, e));
		}
		o(i, 'onDestroy');
		try {
			e._destroy(t || null, i);
		}
		catch (l) {
			i(l);
		}
	}
	o(br, '_destroy');
	function ba(e, t) {
		(Lt(e, t), gr(e));
	}
	o(ba, 'emitErrorCloseNT');
	function gr(e) {
		const t = e._readableState;
		const n = e._writableState;
		(n && (n.closeEmitted = !0),
		t && (t.closeEmitted = !0),
		((n != null && n.emitClose) || (t != null && t.emitClose)) && e.emit('close'));
	}
	o(gr, 'emitCloseNT');
	function Lt(e, t) {
		const n = e._readableState;
		const r = e._writableState;
		(r != null && r.errorEmitted)
		|| (n != null && n.errorEmitted)
		|| (r && (r.errorEmitted = !0), n && (n.errorEmitted = !0), e.emit('error', t));
	}
	o(Lt, 'emitErrorNT');
	function pa() {
		const e = this._readableState;
		const t = this._writableState;
		(e
			&& ((e.constructed = !0),
			(e.closed = !1),
			(e.closeEmitted = !1),
			(e.destroyed = !1),
			(e.errored = null),
			(e.errorEmitted = !1),
			(e.reading = !1),
			(e.ended = e.readable === !1),
			(e.endEmitted = e.readable === !1)),
		t
		&& ((t.constructed = !0),
		(t.destroyed = !1),
		(t.closed = !1),
		(t.closeEmitted = !1),
		(t.errored = null),
		(t.errorEmitted = !1),
		(t.finalCalled = !1),
		(t.prefinished = !1),
		(t.ended = t.writable === !1),
		(t.ending = t.writable === !1),
		(t.finished = t.writable === !1)));
	}
	o(pa, 'undestroy');
	function Pt(e, t, n) {
		const r = e._readableState;
		const i = e._writableState;
		if ((i != null && i.destroyed) || (r != null && r.destroyed))
			return this;
		(r != null && r.autoDestroy) || (i != null && i.autoDestroy)
			? e.destroy(t)
			: t
				&& (t.stack,
				i && !i.errored && (i.errored = t),
				r && !r.errored && (r.errored = t),
				n ? U.nextTick(Lt, e, t) : Lt(e, t));
	}
	o(Pt, 'errorOrDestroy');
	function _a(e, t) {
		if (typeof e._construct != 'function')
			return;
		const n = e._readableState;
		const r = e._writableState;
		(n && (n.constructed = !1),
		r && (r.constructed = !1),
		e.once(Dt, t),
		!(e.listenerCount(Dt) > 1) && U.nextTick(ya, e));
	}
	o(_a, 'construct');
	function ya(e) {
		let t = !1;
		function n(r) {
			if (t) {
				Pt(e, r ?? new aa());
				return;
			}
			t = !0;
			const i = e._readableState;
			const l = e._writableState;
			const a = l || i;
			(i && (i.constructed = !0),
			l && (l.constructed = !0),
			a.destroyed ? e.emit(yr, r) : r ? Pt(e, r, !0) : U.nextTick(wa, e));
		}
		o(n, 'onConstruct');
		try {
			e._construct((r) => {
				U.nextTick(n, r);
			});
		}
		catch (r) {
			U.nextTick(n, r);
		}
	}
	o(ya, 'constructNT');
	function wa(e) {
		e.emit(Dt);
	}
	o(wa, 'emitConstructNT');
	function pr(e) {
		return e?.setHeader && typeof e.abort == 'function';
	}
	o(pr, 'isRequest');
	function Sr(e) {
		e.emit('close');
	}
	o(Sr, 'emitCloseLegacy');
	function ga(e, t) {
		(e.emit('error', t), U.nextTick(Sr, e));
	}
	o(ga, 'emitErrorCloseLegacy');
	function Sa(e, t) {
		!e
		|| sa(e)
		|| (!t && !da(e) && (t = new fa()),
		ca(e)
			? ((e.socket = null), e.destroy(t))
			: pr(e)
				? e.abort()
				: pr(e.req)
					? e.req.abort()
					: typeof e.destroy == 'function'
						? e.destroy(t)
						: typeof e.close == 'function'
							? e.close()
							: t
								? U.nextTick(ga, e, t)
								: U.nextTick(Sr, e),
		e.destroyed || (e[ua] = !0));
	}
	o(Sa, 'destroyer');
	Er.exports = { construct: _a, destroyer: Sa, destroy: ha, undestroy: pa, errorOrDestroy: Pt };
});
const Ve = A((Vs, Ar) => {
	'use strict';
	const { ArrayIsArray: Ea, ObjectSetPrototypeOf: Rr } = T();
	const { EventEmitter: He } = N('events');
	function Ge(e) {
		He.call(this, e);
	}
	o(Ge, 'Stream');
	Rr(Ge.prototype, He.prototype);
	Rr(Ge, He);
	Ge.prototype.pipe = function (e, t) {
		const n = this;
		function r(d) {
			e.writable && e.write(d) === !1 && n.pause && n.pause();
		}
		(o(r, 'ondata'), n.on('data', r));
		function i() {
			n.readable && n.resume && n.resume();
		}
		(o(i, 'ondrain'),
		e.on('drain', i),
		!e._isStdio && (!t || t.end !== !1) && (n.on('end', a), n.on('close', u)));
		let l = !1;
		function a() {
			l || ((l = !0), e.end());
		}
		o(a, 'onend');
		function u() {
			l || ((l = !0), typeof e.destroy == 'function' && e.destroy());
		}
		o(u, 'onclose');
		function s(d) {
			(f(), He.listenerCount(this, 'error') === 0 && this.emit('error', d));
		}
		(o(s, 'onerror'), Ot(n, 'error', s), Ot(e, 'error', s));
		function f() {
			(n.removeListener('data', r),
			e.removeListener('drain', i),
			n.removeListener('end', a),
			n.removeListener('close', u),
			n.removeListener('error', s),
			e.removeListener('error', s),
			n.removeListener('end', f),
			n.removeListener('close', f),
			e.removeListener('close', f));
		}
		return (
			o(f, 'cleanup'), n.on('end', f), n.on('close', f), e.on('close', f), e.emit('pipe', n), e
		);
	};
	function Ot(e, t, n) {
		if (typeof e.prependListener == 'function')
			return e.prependListener(t, n);
		!e._events || !e._events[t]
			? e.on(t, n)
			: Ea(e._events[t])
				? e._events[t].unshift(n)
				: (e._events[t] = [n, e._events[t]]);
	}
	o(Ot, 'prependListener');
	Ar.exports = { Stream: Ge, prependListener: Ot };
});
const Ie = A((Ks, Ye) => {
	'use strict';
	const { SymbolDispose: Ra } = T();
	const { AbortError: mr, codes: Aa } = L();
	const { isNodeStream: Tr, isWebStream: ma, kControllerErrorFunction: Ta } = B();
	const Ia = Y();
	const { ERR_INVALID_ARG_TYPE: Ir } = Aa;
	let Wt;
	const Ma = o((e, t) => {
		if (typeof e != 'object' || !('aborted' in e))
			throw new Ir(t, 'AbortSignal', e);
	}, 'validateAbortSignal');
	Ye.exports.addAbortSignal = o((t, n) => {
		if ((Ma(t, 'signal'), !Tr(n) && !ma(n)))
			throw new Ir('stream', ['ReadableStream', 'WritableStream', 'Stream'], n);
		return Ye.exports.addAbortSignalNoValidate(t, n);
	}, 'addAbortSignal');
	Ye.exports.addAbortSignalNoValidate = function (e, t) {
		if (typeof e != 'object' || !('aborted' in e))
			return t;
		const n = Tr(t)
			? () => {
					t.destroy(new mr(void 0, { cause: e.reason }));
				}
			: () => {
					t[Ta](new mr(void 0, { cause: e.reason }));
				};
		if (e.aborted) {
			n();
		}
		else {
			Wt = Wt || O().addAbortListener;
			const r = Wt(e, n);
			Ia(t, r[Ra]);
		}
		return t;
	};
});
const Nr = A((Js, kr) => {
	'use strict';
	const {
		StringPrototypeSlice: Mr,
		SymbolIterator: ka,
		TypedArrayPrototypeSet: Ke,
		Uint8Array: Na,
	} = T();
	const { Buffer: xt } = N('buffer');
	const { inspect: qa } = O();
	kr.exports = class {
		static {
			o(this, 'BufferList');
		}

		constructor() {
			((this.head = null), (this.tail = null), (this.length = 0));
		}

		push(t) {
			const n = { data: t, next: null };
			(this.length > 0 ? (this.tail.next = n) : (this.head = n), (this.tail = n), ++this.length);
		}

		unshift(t) {
			const n = { data: t, next: this.head };
			(this.length === 0 && (this.tail = n), (this.head = n), ++this.length);
		}

		shift() {
			if (this.length === 0)
				return;
			const t = this.head.data;
			return (
				this.length === 1 ? (this.head = this.tail = null) : (this.head = this.head.next),
				--this.length,
				t
			);
		}

		clear() {
			((this.head = this.tail = null), (this.length = 0));
		}

		join(t) {
			if (this.length === 0)
				return '';
			let n = this.head;
			let r = `${n.data}`;
			for (; (n = n.next) !== null;) r += t + n.data;
			return r;
		}

		concat(t) {
			if (this.length === 0)
				return xt.alloc(0);
			const n = xt.allocUnsafe(t >>> 0);
			let r = this.head;
			let i = 0;
			for (; r;) (Ke(n, r.data, i), (i += r.data.length), (r = r.next));
			return n;
		}

		consume(t, n) {
			const r = this.head.data;
			if (t < r.length) {
				const i = r.slice(0, t);
				return ((this.head.data = r.slice(t)), i);
			}
			return t === r.length ? this.shift() : n ? this._getString(t) : this._getBuffer(t);
		}

		first() {
			return this.head.data;
		}

		* [ka]() {
			for (let t = this.head; t; t = t.next) yield t.data;
		}

		_getString(t) {
			let n = '';
			let r = this.head;
			let i = 0;
			do {
				const l = r.data;
				if (t > l.length) {
					((n += l), (t -= l.length));
				}
				else {
					t === l.length
						? ((n += l), ++i, r.next ? (this.head = r.next) : (this.head = this.tail = null))
						: ((n += Mr(l, 0, t)), (this.head = r), (r.data = Mr(l, t)));
					break;
				}
				++i;
			} while ((r = r.next) !== null);
			return ((this.length -= i), n);
		}

		_getBuffer(t) {
			const n = xt.allocUnsafe(t);
			const r = t;
			let i = this.head;
			let l = 0;
			do {
				const a = i.data;
				if (t > a.length) {
					(Ke(n, a, r - t), (t -= a.length));
				}
				else {
					t === a.length
						? (Ke(n, a, r - t), ++l, i.next ? (this.head = i.next) : (this.head = this.tail = null))
						: (Ke(n, new Na(a.buffer, a.byteOffset, t), r - t),
							(this.head = i),
							(i.data = a.slice(t)));
					break;
				}
				++l;
			} while ((i = i.next) !== null);
			return ((this.length -= l), n);
		}

		[Symbol.for('nodejs.util.inspect.custom')](t, n) {
			return qa(this, { ...n, depth: 0, customInspect: !1 });
		}
	};
});
const Me = A((Zs, Pr) => {
	'use strict';
	const { MathFloor: Da, NumberIsInteger: La } = T();
	const { validateInteger: Pa } = pe();
	const { ERR_INVALID_ARG_VALUE: Oa } = L().codes;
	let qr = 16 * 1024;
	let Dr = 16;
	function Wa(e, t, n) {
		return e.highWaterMark ?? t ? e[n] : null;
	}
	o(Wa, 'highWaterMarkFrom');
	function Lr(e) {
		return e ? Dr : qr;
	}
	o(Lr, 'getDefaultHighWaterMark');
	function xa(e, t) {
		(Pa(t, 'value', 0), e ? (Dr = t) : (qr = t));
	}
	o(xa, 'setDefaultHighWaterMark');
	function Ca(e, t, n, r) {
		const i = Wa(t, r, n);
		if (i != null) {
			if (!La(i) || i < 0) {
				const l = r ? `options.${n}` : 'options.highWaterMark';
				throw new Oa(l, i);
			}
			return Da(i);
		}
		return Lr(e.objectMode);
	}
	o(Ca, 'getHighWaterMark');
	Pr.exports = { getHighWaterMark: Ca, getDefaultHighWaterMark: Lr, setDefaultHighWaterMark: xa };
});
const Ct = A((td, Cr) => {
	'use strict';
	const Or = N('process');
	const { PromisePrototypeThen: $a, SymbolAsyncIterator: Wr, SymbolIterator: xr } = T();
	const { Buffer: ja } = N('buffer');
	const { ERR_INVALID_ARG_TYPE: Fa, ERR_STREAM_NULL_VALUES: va } = L().codes;
	function Ba(e, t, n) {
		let r;
		if (typeof t == 'string' || t instanceof ja) {
			return new e({
				objectMode: !0,
				...n,
				read() {
					(this.push(t), this.push(null));
				},
			});
		}
		let i;
		if (t && t[Wr])
			((i = !0), (r = t[Wr]()));
		else if (t && t[xr])
			((i = !1), (r = t[xr]()));
		else throw new Fa('iterable', ['Iterable'], t);
		const l = new e({ objectMode: !0, highWaterMark: 1, ...n });
		let a = !1;
		((l._read = function () {
			a || ((a = !0), s());
		}),
		(l._destroy = function (f, d) {
			$a(
				u(f),
				() => Or.nextTick(d, f),
				c => Or.nextTick(d, c || f),
			);
		}));
		async function u(f) {
			const d = f != null;
			const c = typeof r.throw == 'function';
			if (d && c) {
				const { value: p, done: h } = await r.throw(f);
				if ((await p, h))
					return;
			}
			if (typeof r.return == 'function') {
				const { value: p } = await r.return();
				await p;
			}
		}
		o(u, 'close');
		async function s() {
			for (;;) {
				try {
					const { value: f, done: d } = i ? await r.next() : r.next();
					if (d) {
						l.push(null);
					}
					else {
						const c = f && typeof f.then == 'function' ? await f : f;
						if (c === null)
							throw ((a = !1), new va());
						if (l.push(c))
							continue;
						a = !1;
					}
				}
				catch (f) {
					l.destroy(f);
				}
				break;
			}
		}
		return (o(s, 'next'), l);
	}
	o(Ba, 'from');
	Cr.exports = Ba;
});
const Ne = A((rd, ti) => {
	'use strict';
	const j = N('process');
	const {
		ArrayPrototypeIndexOf: Ua,
		NumberIsInteger: Ha,
		NumberIsNaN: Ga,
		NumberParseInt: Va,
		ObjectDefineProperties: Gt,
		ObjectKeys: Ya,
		ObjectSetPrototypeOf: Fr,
		Promise: vr,
		SafeSet: Ka,
		SymbolAsyncDispose: za,
		SymbolAsyncIterator: Xa,
		Symbol: Ja,
	} = T();
	ti.exports = _;
	_.ReadableState = Qe;
	const { EventEmitter: Qa } = N('events');
	const { Stream: Z, prependListener: Za } = Ve();
	const { Buffer: $t } = N('buffer');
	const { addAbortSignal: ef } = Ie();
	const Br = Y();
	var w = O().debuglog('stream', (e) => {
		w = e;
	});
	const tf = Nr();
	const ge = oe();
	const { getHighWaterMark: nf, getDefaultHighWaterMark: rf } = Me();
	const {
		aggregateTwoErrors: $r,
		codes: {
			ERR_INVALID_ARG_TYPE: of,
			ERR_METHOD_NOT_IMPLEMENTED: lf,
			ERR_OUT_OF_RANGE: af,
			ERR_STREAM_PUSH_AFTER_EOF: ff,
			ERR_STREAM_UNSHIFT_AFTER_END_EVENT: uf,
		},
		AbortError: sf,
	} = L();
	const { validateObject: df } = pe();
	const le = Ja('kPaused');
	const { StringDecoder: Ur } = N('string_decoder/');
	const cf = Ct();
	Fr(_.prototype, Z.prototype);
	Fr(_, Z);
	const jt = o(() => {}, 'nop');
	const { errorOrDestroy: ye } = ge;
	const we = 1;
	const hf = 2;
	const Hr = 4;
	const ke = 8;
	const Gr = 16;
	const ze = 32;
	const Xe = 64;
	const Vr = 128;
	const bf = 256;
	const pf = 512;
	const _f = 1024;
	const Ut = 2048;
	const Ht = 4096;
	const yf = 8192;
	const wf = 16384;
	const gf = 32768;
	const Yr = 65536;
	const Sf = 1 << 17;
	const Ef = 1 << 18;
	function q(e) {
		return {
			enumerable: !1,
			get() {
				return (this.state & e) !== 0;
			},
			set(t) {
				t ? (this.state |= e) : (this.state &= ~e);
			},
		};
	}
	o(q, 'makeBitMapDescriptor');
	Gt(Qe.prototype, {
		objectMode: q(we),
		ended: q(hf),
		endEmitted: q(Hr),
		reading: q(ke),
		constructed: q(Gr),
		sync: q(ze),
		needReadable: q(Xe),
		emittedReadable: q(Vr),
		readableListening: q(bf),
		resumeScheduled: q(pf),
		errorEmitted: q(_f),
		emitClose: q(Ut),
		autoDestroy: q(Ht),
		destroyed: q(yf),
		closed: q(wf),
		closeEmitted: q(gf),
		multiAwaitDrain: q(Yr),
		readingMore: q(Sf),
		dataEmitted: q(Ef),
	});
	function Qe(e, t, n) {
		(typeof n != 'boolean' && (n = t instanceof H()),
		(this.state = Ut | Ht | Gr | ze),
		e && e.objectMode && (this.state |= we),
		n && e && e.readableObjectMode && (this.state |= we),
		(this.highWaterMark = e ? nf(this, e, 'readableHighWaterMark', n) : rf(!1)),
		(this.buffer = new tf()),
		(this.length = 0),
		(this.pipes = []),
		(this.flowing = null),
		(this[le] = null),
		e && e.emitClose === !1 && (this.state &= ~Ut),
		e && e.autoDestroy === !1 && (this.state &= ~Ht),
		(this.errored = null),
		(this.defaultEncoding = (e && e.defaultEncoding) || 'utf8'),
		(this.awaitDrainWriters = null),
		(this.decoder = null),
		(this.encoding = null),
		e && e.encoding && ((this.decoder = new Ur(e.encoding)), (this.encoding = e.encoding)));
	}
	o(Qe, 'ReadableState');
	function _(e) {
		if (!(this instanceof _))
			return new _(e);
		const t = this instanceof H();
		((this._readableState = new Qe(e, this, t)),
		e
		&& (typeof e.read == 'function' && (this._read = e.read),
		typeof e.destroy == 'function' && (this._destroy = e.destroy),
		typeof e.construct == 'function' && (this._construct = e.construct),
		e.signal && !t && ef(e.signal, this)),
		Z.call(this, e),
		ge.construct(this, () => {
			this._readableState.needReadable && Je(this, this._readableState);
		}));
	}
	o(_, 'Readable');
	_.prototype.destroy = ge.destroy;
	_.prototype._undestroy = ge.undestroy;
	_.prototype._destroy = function (e, t) {
		t(e);
	};
	_.prototype[Qa.captureRejectionSymbol] = function (e) {
		this.destroy(e);
	};
	_.prototype[za] = function () {
		let e;
		return (
			this.destroyed || ((e = this.readableEnded ? null : new sf()), this.destroy(e)),
			new vr((t, n) => Br(this, r => (r && r !== e ? n(r) : t(null))))
		);
	};
	_.prototype.push = function (e, t) {
		return Kr(this, e, t, !1);
	};
	_.prototype.unshift = function (e, t) {
		return Kr(this, e, t, !0);
	};
	function Kr(e, t, n, r) {
		w('readableAddChunk', t);
		const i = e._readableState;
		let l;
		if (
			((i.state & we) === 0
				&& (typeof t == 'string'
					? ((n = n || i.defaultEncoding),
						i.encoding !== n
						&& (r && i.encoding
							? (t = $t.from(t, n).toString(i.encoding))
							: ((t = $t.from(t, n)), (n = ''))))
					: t instanceof $t
						? (n = '')
						: Z._isUint8Array(t)
							? ((t = Z._uint8ArrayToBuffer(t)), (n = ''))
							: t != null && (l = new of('chunk', ['string', 'Buffer', 'Uint8Array'], t))),
			l)) {
			ye(e, l);
		}
		else if (t === null) {
			((i.state &= ~ke), mf(e, i));
		}
		else if ((i.state & we) !== 0 || (t && t.length > 0)) {
			if (r) {
				if ((i.state & Hr) !== 0) {
					ye(e, new uf());
				}
				else {
					if (i.destroyed || i.errored)
						return !1;
					Ft(e, i, t, !0);
				}
			}
			else if (i.ended) {
				ye(e, new ff());
			}
			else {
				if (i.destroyed || i.errored)
					return !1;
				((i.state &= ~ke),
				i.decoder && !n
					? ((t = i.decoder.write(t)),
						i.objectMode || t.length !== 0 ? Ft(e, i, t, !1) : Je(e, i))
					: Ft(e, i, t, !1));
			}
		}
		else {
			r || ((i.state &= ~ke), Je(e, i));
		}
		return !i.ended && (i.length < i.highWaterMark || i.length === 0);
	}
	o(Kr, 'readableAddChunk');
	function Ft(e, t, n, r) {
		(t.flowing && t.length === 0 && !t.sync && e.listenerCount('data') > 0
			? ((t.state & Yr) !== 0 ? t.awaitDrainWriters.clear() : (t.awaitDrainWriters = null),
				(t.dataEmitted = !0),
				e.emit('data', n))
			: ((t.length += t.objectMode ? 1 : n.length),
				r ? t.buffer.unshift(n) : t.buffer.push(n),
				(t.state & Xe) !== 0 && Ze(e)),
		Je(e, t));
	}
	o(Ft, 'addChunk');
	_.prototype.isPaused = function () {
		const e = this._readableState;
		return e[le] === !0 || e.flowing === !1;
	};
	_.prototype.setEncoding = function (e) {
		const t = new Ur(e);
		((this._readableState.decoder = t),
		(this._readableState.encoding = this._readableState.decoder.encoding));
		const n = this._readableState.buffer;
		let r = '';
		for (const i of n) r += t.write(i);
		return (n.clear(), r !== '' && n.push(r), (this._readableState.length = r.length), this);
	};
	const Rf = 1073741824;
	function Af(e) {
		if (e > Rf)
			throw new af('size', '<= 1GiB', e);
		return (
			e--, (e |= e >>> 1), (e |= e >>> 2), (e |= e >>> 4), (e |= e >>> 8), (e |= e >>> 16), e++, e
		);
	}
	o(Af, 'computeNewHighWaterMark');
	function jr(e, t) {
		return e <= 0 || (t.length === 0 && t.ended)
			? 0
			: (t.state & we) !== 0
					? 1
					: Ga(e)
						? t.flowing && t.length
							? t.buffer.first().length
							: t.length
						: e <= t.length
							? e
							: t.ended
								? t.length
								: 0;
	}
	o(jr, 'howMuchToRead');
	_.prototype.read = function (e) {
		(w('read', e), e === void 0 ? (e = Number.NaN) : Ha(e) || (e = Va(e, 10)));
		const t = this._readableState;
		const n = e;
		if (
			(e > t.highWaterMark && (t.highWaterMark = Af(e)),
			e !== 0 && (t.state &= ~Vr),
			e === 0
			&& t.needReadable
			&& ((t.highWaterMark !== 0 ? t.length >= t.highWaterMark : t.length > 0) || t.ended))) {
			return (
				w('read: emitReadable', t.length, t.ended),
				t.length === 0 && t.ended ? vt(this) : Ze(this),
				null
			);
		}
		if (((e = jr(e, t)), e === 0 && t.ended))
			return (t.length === 0 && vt(this), null);
		let r = (t.state & Xe) !== 0;
		if (
			(w('need readable', r),
			(t.length === 0 || t.length - e < t.highWaterMark)
			&& ((r = !0), w('length less than watermark', r)),
			t.ended || t.reading || t.destroyed || t.errored || !t.constructed)) {
			((r = !1), w('reading, ended or constructing', r));
		}
		else if (r) {
			(w('do read'), (t.state |= ke | ze), t.length === 0 && (t.state |= Xe));
			try {
				this._read(t.highWaterMark);
			}
			catch (l) {
				ye(this, l);
			}
			((t.state &= ~ze), t.reading || (e = jr(n, t)));
		}
		let i;
		return (
			e > 0 ? (i = Zr(e, t)) : (i = null),
			i === null
				? ((t.needReadable = t.length <= t.highWaterMark), (e = 0))
				: ((t.length -= e),
					t.multiAwaitDrain ? t.awaitDrainWriters.clear() : (t.awaitDrainWriters = null)),
			t.length === 0 && (t.ended || (t.needReadable = !0), n !== e && t.ended && vt(this)),
			i !== null
			&& !t.errorEmitted
			&& !t.closeEmitted
			&& ((t.dataEmitted = !0), this.emit('data', i)),
			i
		);
	};
	function mf(e, t) {
		if ((w('onEofChunk'), !t.ended)) {
			if (t.decoder) {
				const n = t.decoder.end();
				n && n.length && (t.buffer.push(n), (t.length += t.objectMode ? 1 : n.length));
			}
			((t.ended = !0), t.sync ? Ze(e) : ((t.needReadable = !1), (t.emittedReadable = !0), zr(e)));
		}
	}
	o(mf, 'onEofChunk');
	function Ze(e) {
		const t = e._readableState;
		(w('emitReadable', t.needReadable, t.emittedReadable),
		(t.needReadable = !1),
		t.emittedReadable
		|| (w('emitReadable', t.flowing), (t.emittedReadable = !0), j.nextTick(zr, e)));
	}
	o(Ze, 'emitReadable');
	function zr(e) {
		const t = e._readableState;
		(w('emitReadable_', t.destroyed, t.length, t.ended),
		!t.destroyed
		&& !t.errored
		&& (t.length || t.ended)
		&& (e.emit('readable'), (t.emittedReadable = !1)),
		(t.needReadable = !t.flowing && !t.ended && t.length <= t.highWaterMark),
		Jr(e));
	}
	o(zr, 'emitReadable_');
	function Je(e, t) {
		!t.readingMore && t.constructed && ((t.readingMore = !0), j.nextTick(Tf, e, t));
	}
	o(Je, 'maybeReadMore');
	function Tf(e, t) {
		for (
			;
			!t.reading && !t.ended && (t.length < t.highWaterMark || (t.flowing && t.length === 0));
		) {
			const n = t.length;
			if ((w('maybeReadMore read 0'), e.read(0), n === t.length))
				break;
		}
		t.readingMore = !1;
	}
	o(Tf, 'maybeReadMore_');
	_.prototype._read = function (e) {
		throw new lf('_read()');
	};
	_.prototype.pipe = function (e, t) {
		const n = this;
		const r = this._readableState;
		(r.pipes.length === 1
			&& (r.multiAwaitDrain
				|| ((r.multiAwaitDrain = !0),
				(r.awaitDrainWriters = new Ka(r.awaitDrainWriters ? [r.awaitDrainWriters] : [])))),
		r.pipes.push(e),
		w('pipe count=%d opts=%j', r.pipes.length, t));
		const l = (!t || t.end !== !1) && e !== j.stdout && e !== j.stderr ? u : E;
		(r.endEmitted ? j.nextTick(l) : n.once('end', l), e.on('unpipe', a));
		function a(y, R) {
			(w('onunpipe'), y === n && R && R.hasUnpiped === !1 && ((R.hasUnpiped = !0), d()));
		}
		o(a, 'onunpipe');
		function u() {
			(w('onend'), e.end());
		}
		o(u, 'onend');
		let s;
		let f = !1;
		function d() {
			(w('cleanup'),
			e.removeListener('close', S),
			e.removeListener('finish', b),
			s && e.removeListener('drain', s),
			e.removeListener('error', h),
			e.removeListener('unpipe', a),
			n.removeListener('end', u),
			n.removeListener('end', E),
			n.removeListener('data', p),
			(f = !0),
			s && r.awaitDrainWriters && (!e._writableState || e._writableState.needDrain) && s());
		}
		o(d, 'cleanup');
		function c() {
			(f
				|| (r.pipes.length === 1 && r.pipes[0] === e
					? (w('false write response, pause', 0),
						(r.awaitDrainWriters = e),
						(r.multiAwaitDrain = !1))
					: r.pipes.length > 1
						&& r.pipes.includes(e)
						&& (w('false write response, pause', r.awaitDrainWriters.size),
						r.awaitDrainWriters.add(e)),
				n.pause()),
			s || ((s = If(n, e)), e.on('drain', s)));
		}
		(o(c, 'pause'), n.on('data', p));
		function p(y) {
			w('ondata');
			const R = e.write(y);
			(w('dest.write', R), R === !1 && c());
		}
		o(p, 'ondata');
		function h(y) {
			if ((w('onerror', y), E(), e.removeListener('error', h), e.listenerCount('error') === 0)) {
				const R = e._writableState || e._readableState;
				R && !R.errorEmitted ? ye(e, y) : e.emit('error', y);
			}
		}
		(o(h, 'onerror'), Za(e, 'error', h));
		function S() {
			(e.removeListener('finish', b), E());
		}
		(o(S, 'onclose'), e.once('close', S));
		function b() {
			(w('onfinish'), e.removeListener('close', S), E());
		}
		(o(b, 'onfinish'), e.once('finish', b));
		function E() {
			(w('unpipe'), n.unpipe(e));
		}
		return (
			o(E, 'unpipe'),
			e.emit('pipe', n),
			e.writableNeedDrain === !0 ? c() : r.flowing || (w('pipe resume'), n.resume()),
			e
		);
	};
	function If(e, t) {
		return o(() => {
			const r = e._readableState;
			(r.awaitDrainWriters === t
				? (w('pipeOnDrain', 1), (r.awaitDrainWriters = null))
				: r.multiAwaitDrain
					&& (w('pipeOnDrain', r.awaitDrainWriters.size), r.awaitDrainWriters.delete(t)),
			(!r.awaitDrainWriters || r.awaitDrainWriters.size === 0)
			&& e.listenerCount('data')
			&& e.resume());
		}, 'pipeOnDrainFunctionResult');
	}
	o(If, 'pipeOnDrain');
	_.prototype.unpipe = function (e) {
		const t = this._readableState;
		const n = { hasUnpiped: !1 };
		if (t.pipes.length === 0)
			return this;
		if (!e) {
			const i = t.pipes;
			((t.pipes = []), this.pause());
			for (let l = 0; l < i.length; l++) i[l].emit('unpipe', this, { hasUnpiped: !1 });
			return this;
		}
		const r = Ua(t.pipes, e);
		return r === -1
			? this
			: (t.pipes.splice(r, 1),
				t.pipes.length === 0 && this.pause(),
				e.emit('unpipe', this, n),
				this);
	};
	_.prototype.on = function (e, t) {
		const n = Z.prototype.on.call(this, e, t);
		const r = this._readableState;
		return (
			e === 'data'
				? ((r.readableListening = this.listenerCount('readable') > 0),
					r.flowing !== !1 && this.resume())
				: e === 'readable'
					&& !r.endEmitted
					&& !r.readableListening
					&& ((r.readableListening = r.needReadable = !0),
					(r.flowing = !1),
					(r.emittedReadable = !1),
					w('on readable', r.length, r.reading),
					r.length ? Ze(this) : r.reading || j.nextTick(Mf, this)),
			n
		);
	};
	_.prototype.addListener = _.prototype.on;
	_.prototype.removeListener = function (e, t) {
		const n = Z.prototype.removeListener.call(this, e, t);
		return (e === 'readable' && j.nextTick(Xr, this), n);
	};
	_.prototype.off = _.prototype.removeListener;
	_.prototype.removeAllListeners = function (e) {
		const t = Z.prototype.removeAllListeners.apply(this, arguments);
		return ((e === 'readable' || e === void 0) && j.nextTick(Xr, this), t);
	};
	function Xr(e) {
		const t = e._readableState;
		((t.readableListening = e.listenerCount('readable') > 0),
		t.resumeScheduled && t[le] === !1
			? (t.flowing = !0)
			: e.listenerCount('data') > 0
				? e.resume()
				: t.readableListening || (t.flowing = null));
	}
	o(Xr, 'updateReadableListening');
	function Mf(e) {
		(w('readable nexttick read 0'), e.read(0));
	}
	o(Mf, 'nReadingNextTick');
	_.prototype.resume = function () {
		const e = this._readableState;
		return (
			e.flowing || (w('resume'), (e.flowing = !e.readableListening), kf(this, e)),
			(e[le] = !1),
			this
		);
	};
	function kf(e, t) {
		t.resumeScheduled || ((t.resumeScheduled = !0), j.nextTick(Nf, e, t));
	}
	o(kf, 'resume');
	function Nf(e, t) {
		(w('resume', t.reading),
		t.reading || e.read(0),
		(t.resumeScheduled = !1),
		e.emit('resume'),
		Jr(e),
		t.flowing && !t.reading && e.read(0));
	}
	o(Nf, 'resume_');
	_.prototype.pause = function () {
		return (
			w('call pause flowing=%j', this._readableState.flowing),
			this._readableState.flowing !== !1
			&& (w('pause'), (this._readableState.flowing = !1), this.emit('pause')),
			(this._readableState[le] = !0),
			this
		);
	};
	function Jr(e) {
		const t = e._readableState;
		for (w('flow', t.flowing); t.flowing && e.read() !== null;);
	}
	o(Jr, 'flow');
	_.prototype.wrap = function (e) {
		let t = !1;
		(e.on('data', (r) => {
			!this.push(r) && e.pause && ((t = !0), e.pause());
		}),
		e.on('end', () => {
			this.push(null);
		}),
		e.on('error', (r) => {
			ye(this, r);
		}),
		e.on('close', () => {
			this.destroy();
		}),
		e.on('destroy', () => {
			this.destroy();
		}),
		(this._read = () => {
			t && e.resume && ((t = !1), e.resume());
		}));
		const n = Ya(e);
		for (let r = 1; r < n.length; r++) {
			const i = n[r];
			this[i] === void 0 && typeof e[i] == 'function' && (this[i] = e[i].bind(e));
		}
		return this;
	};
	_.prototype[Xa] = function () {
		return Qr(this);
	};
	_.prototype.iterator = function (e) {
		return (e !== void 0 && df(e, 'options'), Qr(this, e));
	};
	function Qr(e, t) {
		typeof e.read != 'function' && (e = _.wrap(e, { objectMode: !0 }));
		const n = qf(e, t);
		return ((n.stream = e), n);
	}
	o(Qr, 'streamToAsyncIterator');
	async function* qf(e, t) {
		let n = jt;
		function r(a) {
			this === e ? (n(), (n = jt)) : (n = a);
		}
		(o(r, 'next'), e.on('readable', r));
		let i;
		const l = Br(e, { writable: !1 }, (a) => {
			((i = a ? $r(i, a) : null), n(), (n = jt));
		});
		try {
			for (;;) {
				const a = e.destroyed ? null : e.read();
				if (a !== null) {
					yield a;
				}
				else {
					if (i)
						throw i;
					if (i === null)
						return;
					await new vr(r);
				}
			}
		}
		catch (a) {
			throw ((i = $r(i, a)), i);
		}
		finally {
			(i || t?.destroyOnReturn !== !1) && (i === void 0 || e._readableState.autoDestroy)
				? ge.destroyer(e, null)
				: (e.off('readable', r), l());
		}
	}
	o(qf, 'createAsyncIterator');
	Gt(_.prototype, {
		readable: {
			__proto__: null,
			get() {
				const e = this._readableState;
				return !!e && e.readable !== !1 && !e.destroyed && !e.errorEmitted && !e.endEmitted;
			},
			set(e) {
				this._readableState && (this._readableState.readable = !!e);
			},
		},
		readableDidRead: {
			__proto__: null,
			enumerable: !1,
			get: o(function () {
				return this._readableState.dataEmitted;
			}, 'get'),
		},
		readableAborted: {
			__proto__: null,
			enumerable: !1,
			get: o(function () {
				return !!(
					this._readableState.readable !== !1
					&& (this._readableState.destroyed || this._readableState.errored)
					&& !this._readableState.endEmitted
				);
			}, 'get'),
		},
		readableHighWaterMark: {
			__proto__: null,
			enumerable: !1,
			get: o(function () {
				return this._readableState.highWaterMark;
			}, 'get'),
		},
		readableBuffer: {
			__proto__: null,
			enumerable: !1,
			get: o(function () {
				return this._readableState && this._readableState.buffer;
			}, 'get'),
		},
		readableFlowing: {
			__proto__: null,
			enumerable: !1,
			get: o(function () {
				return this._readableState.flowing;
			}, 'get'),
			set: o(function (e) {
				this._readableState && (this._readableState.flowing = e);
			}, 'set'),
		},
		readableLength: {
			__proto__: null,
			enumerable: !1,
			get() {
				return this._readableState.length;
			},
		},
		readableObjectMode: {
			__proto__: null,
			enumerable: !1,
			get() {
				return this._readableState ? this._readableState.objectMode : !1;
			},
		},
		readableEncoding: {
			__proto__: null,
			enumerable: !1,
			get() {
				return this._readableState ? this._readableState.encoding : null;
			},
		},
		errored: {
			__proto__: null,
			enumerable: !1,
			get() {
				return this._readableState ? this._readableState.errored : null;
			},
		},
		closed: {
			__proto__: null,
			get() {
				return this._readableState ? this._readableState.closed : !1;
			},
		},
		destroyed: {
			__proto__: null,
			enumerable: !1,
			get() {
				return this._readableState ? this._readableState.destroyed : !1;
			},
			set(e) {
				this._readableState && (this._readableState.destroyed = e);
			},
		},
		readableEnded: {
			__proto__: null,
			enumerable: !1,
			get() {
				return this._readableState ? this._readableState.endEmitted : !1;
			},
		},
	});
	Gt(Qe.prototype, {
		pipesCount: {
			__proto__: null,
			get() {
				return this.pipes.length;
			},
		},
		paused: {
			__proto__: null,
			get() {
				return this[le] !== !1;
			},
			set(e) {
				this[le] = !!e;
			},
		},
	});
	_._fromList = Zr;
	function Zr(e, t) {
		if (t.length === 0)
			return null;
		let n;
		return (
			t.objectMode
				? (n = t.buffer.shift())
				: !e || e >= t.length
						? (t.decoder
								? (n = t.buffer.join(''))
								: t.buffer.length === 1
									? (n = t.buffer.first())
									: (n = t.buffer.concat(t.length)),
							t.buffer.clear())
						: (n = t.buffer.consume(e, t.decoder)),
			n
		);
	}
	o(Zr, 'fromList');
	function vt(e) {
		const t = e._readableState;
		(w('endReadable', t.endEmitted), t.endEmitted || ((t.ended = !0), j.nextTick(Df, t, e)));
	}
	o(vt, 'endReadable');
	function Df(e, t) {
		if (
			(w('endReadableNT', e.endEmitted, e.length),
			!e.errored && !e.closeEmitted && !e.endEmitted && e.length === 0)) {
			if (((e.endEmitted = !0), t.emit('end'), t.writable && t.allowHalfOpen === !1)) {
				j.nextTick(Lf, t);
			}
			else if (e.autoDestroy) {
				const n = t._writableState;
				(!n || (n.autoDestroy && (n.finished || n.writable === !1))) && t.destroy();
			}
		}
	}
	o(Df, 'endReadableNT');
	function Lf(e) {
		e.writable && !e.writableEnded && !e.destroyed && e.end();
	}
	o(Lf, 'endWritableNT');
	_.from = function (e, t) {
		return cf(_, e, t);
	};
	let Bt;
	function ei() {
		return (Bt === void 0 && (Bt = {}), Bt);
	}
	o(ei, 'lazyWebStreams');
	_.fromWeb = function (e, t) {
		return ei().newStreamReadableFromReadableStream(e, t);
	};
	_.toWeb = function (e, t) {
		return ei().newReadableStreamFromStreamReadable(e, t);
	};
	_.wrap = function (e, t) {
		let n, r;
		return new _({
			objectMode:
        (n = (r = e.readableObjectMode) !== null && r !== void 0 ? r : e.objectMode) !== null
        && n !== void 0
        	? n
        	: !0,
			...t,
			destroy(i, l) {
				(ge.destroyer(e, i), l(i));
			},
		}).wrap(e);
	};
});
const it = A((od, hi) => {
	'use strict';
	const ae = N('process');
	const {
		ArrayPrototypeSlice: ii,
		Error: Pf,
		FunctionPrototypeSymbolHasInstance: oi,
		ObjectDefineProperty: li,
		ObjectDefineProperties: Of,
		ObjectSetPrototypeOf: ai,
		StringPrototypeToLowerCase: Wf,
		Symbol: xf,
		SymbolHasInstance: Cf,
	} = T();
	hi.exports = I;
	I.WritableState = Le;
	const { EventEmitter: $f } = N('events');
	const qe = Ve().Stream;
	const { Buffer: et } = N('buffer');
	const rt = oe();
	const { addAbortSignal: jf } = Ie();
	const { getHighWaterMark: Ff, getDefaultHighWaterMark: vf } = Me();
	const {
		ERR_INVALID_ARG_TYPE: Bf,
		ERR_METHOD_NOT_IMPLEMENTED: Uf,
		ERR_MULTIPLE_CALLBACK: fi,
		ERR_STREAM_CANNOT_PIPE: Hf,
		ERR_STREAM_DESTROYED: De,
		ERR_STREAM_ALREADY_FINISHED: Gf,
		ERR_STREAM_NULL_VALUES: Vf,
		ERR_STREAM_WRITE_AFTER_END: Yf,
		ERR_UNKNOWN_ENCODING: ui,
	} = L().codes;
	const { errorOrDestroy: Se } = rt;
	ai(I.prototype, qe.prototype);
	ai(I, qe);
	function Kt() {}
	o(Kt, 'nop');
	const Ee = xf('kOnFinished');
	function Le(e, t, n) {
		(typeof n != 'boolean' && (n = t instanceof H()),
		(this.objectMode = !!(e && e.objectMode)),
		n && (this.objectMode = this.objectMode || !!(e && e.writableObjectMode)),
		(this.highWaterMark = e ? Ff(this, e, 'writableHighWaterMark', n) : vf(!1)),
		(this.finalCalled = !1),
		(this.needDrain = !1),
		(this.ending = !1),
		(this.ended = !1),
		(this.finished = !1),
		(this.destroyed = !1));
		const r = !!(e && e.decodeStrings === !1);
		((this.decodeStrings = !r),
		(this.defaultEncoding = (e && e.defaultEncoding) || 'utf8'),
		(this.length = 0),
		(this.writing = !1),
		(this.corked = 0),
		(this.sync = !0),
		(this.bufferProcessing = !1),
		(this.onwrite = zf.bind(void 0, t)),
		(this.writecb = null),
		(this.writelen = 0),
		(this.afterWriteTickInfo = null),
		nt(this),
		(this.pendingcb = 0),
		(this.constructed = !0),
		(this.prefinished = !1),
		(this.errorEmitted = !1),
		(this.emitClose = !e || e.emitClose !== !1),
		(this.autoDestroy = !e || e.autoDestroy !== !1),
		(this.errored = null),
		(this.closed = !1),
		(this.closeEmitted = !1),
		(this[Ee] = []));
	}
	o(Le, 'WritableState');
	function nt(e) {
		((e.buffered = []), (e.bufferedIndex = 0), (e.allBuffers = !0), (e.allNoop = !0));
	}
	o(nt, 'resetBuffer');
	Le.prototype.getBuffer = o(function () {
		return ii(this.buffered, this.bufferedIndex);
	}, 'getBuffer');
	li(Le.prototype, 'bufferedRequestCount', {
		__proto__: null,
		get() {
			return this.buffered.length - this.bufferedIndex;
		},
	});
	function I(e) {
		const t = this instanceof H();
		if (!t && !oi(I, this))
			return new I(e);
		((this._writableState = new Le(e, this, t)),
		e
		&& (typeof e.write == 'function' && (this._write = e.write),
		typeof e.writev == 'function' && (this._writev = e.writev),
		typeof e.destroy == 'function' && (this._destroy = e.destroy),
		typeof e.final == 'function' && (this._final = e.final),
		typeof e.construct == 'function' && (this._construct = e.construct),
		e.signal && jf(e.signal, this)),
		qe.call(this, e),
		rt.construct(this, () => {
			const n = this._writableState;
			(n.writing || Xt(this, n), Jt(this, n));
		}));
	}
	o(I, 'Writable');
	li(I, Cf, {
		__proto__: null,
		value: o(function (e) {
			return oi(this, e) ? !0 : this !== I ? !1 : e && e._writableState instanceof Le;
		}, 'value'),
	});
	I.prototype.pipe = function () {
		Se(this, new Hf());
	};
	function si(e, t, n, r) {
		const i = e._writableState;
		if (typeof n == 'function') {
			((r = n), (n = i.defaultEncoding));
		}
		else {
			if (!n)
				n = i.defaultEncoding;
			else if (n !== 'buffer' && !et.isEncoding(n))
				throw new ui(n);
			typeof r != 'function' && (r = Kt);
		}
		if (t === null)
			throw new Vf();
		if (!i.objectMode) {
			if (typeof t == 'string')
				i.decodeStrings !== !1 && ((t = et.from(t, n)), (n = 'buffer'));
			else if (t instanceof et)
				n = 'buffer';
			else if (qe._isUint8Array(t))
				((t = qe._uint8ArrayToBuffer(t)), (n = 'buffer'));
			else throw new Bf('chunk', ['string', 'Buffer', 'Uint8Array'], t);
		}
		let l;
		return (
			i.ending ? (l = new Yf()) : i.destroyed && (l = new De('write')),
			l ? (ae.nextTick(r, l), Se(e, l, !0), l) : (i.pendingcb++, Kf(e, i, t, n, r))
		);
	}
	o(si, '_write');
	I.prototype.write = function (e, t, n) {
		return si(this, e, t, n) === !0;
	};
	I.prototype.cork = function () {
		this._writableState.corked++;
	};
	I.prototype.uncork = function () {
		const e = this._writableState;
		e.corked && (e.corked--, e.writing || Xt(this, e));
	};
	I.prototype.setDefaultEncoding = o(function (t) {
		if ((typeof t == 'string' && (t = Wf(t)), !et.isEncoding(t)))
			throw new ui(t);
		return ((this._writableState.defaultEncoding = t), this);
	}, 'setDefaultEncoding');
	function Kf(e, t, n, r, i) {
		const l = t.objectMode ? 1 : n.length;
		t.length += l;
		const a = t.length < t.highWaterMark;
		return (
			a || (t.needDrain = !0),
			t.writing || t.corked || t.errored || !t.constructed
				? (t.buffered.push({ chunk: n, encoding: r, callback: i }),
					t.allBuffers && r !== 'buffer' && (t.allBuffers = !1),
					t.allNoop && i !== Kt && (t.allNoop = !1))
				: ((t.writelen = l),
					(t.writecb = i),
					(t.writing = !0),
					(t.sync = !0),
					e._write(n, r, t.onwrite),
					(t.sync = !1)),
			a && !t.errored && !t.destroyed
		);
	}
	o(Kf, 'writeOrBuffer');
	function ni(e, t, n, r, i, l, a) {
		((t.writelen = r),
		(t.writecb = a),
		(t.writing = !0),
		(t.sync = !0),
		t.destroyed
			? t.onwrite(new De('write'))
			: n
				? e._writev(i, t.onwrite)
				: e._write(i, l, t.onwrite),
		(t.sync = !1));
	}
	o(ni, 'doWrite');
	function ri(e, t, n, r) {
		(--t.pendingcb, r(n), zt(t), Se(e, n));
	}
	o(ri, 'onwriteError');
	function zf(e, t) {
		const n = e._writableState;
		const r = n.sync;
		const i = n.writecb;
		if (typeof i != 'function') {
			Se(e, new fi());
			return;
		}
		((n.writing = !1),
		(n.writecb = null),
		(n.length -= n.writelen),
		(n.writelen = 0),
		t
			? (t.stack,
				n.errored || (n.errored = t),
				e._readableState && !e._readableState.errored && (e._readableState.errored = t),
				r ? ae.nextTick(ri, e, n, t, i) : ri(e, n, t, i))
			: (n.buffered.length > n.bufferedIndex && Xt(e, n),
				r
					? n.afterWriteTickInfo !== null && n.afterWriteTickInfo.cb === i
						? n.afterWriteTickInfo.count++
						: ((n.afterWriteTickInfo = { count: 1, cb: i, stream: e, state: n }),
							ae.nextTick(Xf, n.afterWriteTickInfo))
					: di(e, n, 1, i)));
	}
	o(zf, 'onwrite');
	function Xf({ stream: e, state: t, count: n, cb: r }) {
		return ((t.afterWriteTickInfo = null), di(e, t, n, r));
	}
	o(Xf, 'afterWriteTick');
	function di(e, t, n, r) {
		for (
			!t.ending
			&& !e.destroyed
			&& t.length === 0
			&& t.needDrain
			&& ((t.needDrain = !1), e.emit('drain'));
			n-- > 0;
		)
			(t.pendingcb--, r());
		(t.destroyed && zt(t), Jt(e, t));
	}
	o(di, 'afterWrite');
	function zt(e) {
		if (e.writing)
			return;
		for (let i = e.bufferedIndex; i < e.buffered.length; ++i) {
			var t;
			const { chunk: l, callback: a } = e.buffered[i];
			const u = e.objectMode ? 1 : l.length;
			((e.length -= u), a((t = e.errored) !== null && t !== void 0 ? t : new De('write')));
		}
		const n = e[Ee].splice(0);
		for (let i = 0; i < n.length; i++) {
			var r;
			n[i]((r = e.errored) !== null && r !== void 0 ? r : new De('end'));
		}
		nt(e);
	}
	o(zt, 'errorBuffer');
	function Xt(e, t) {
		if (t.corked || t.bufferProcessing || t.destroyed || !t.constructed)
			return;
		const { buffered: n, bufferedIndex: r, objectMode: i } = t;
		const l = n.length - r;
		if (!l)
			return;
		let a = r;
		if (((t.bufferProcessing = !0), l > 1 && e._writev)) {
			t.pendingcb -= l - 1;
			const u = t.allNoop
				? Kt
				: (f) => {
						for (let d = a; d < n.length; ++d) n[d].callback(f);
					};
			const s = t.allNoop && a === 0 ? n : ii(n, a);
			((s.allBuffers = t.allBuffers), ni(e, t, !0, t.length, s, '', u), nt(t));
		}
		else {
			do {
				const { chunk: u, encoding: s, callback: f } = n[a];
				n[a++] = null;
				const d = i ? 1 : u.length;
				ni(e, t, !1, d, u, s, f);
			} while (a < n.length && !t.writing);
			a === n.length
				? nt(t)
				: a > 256
					? (n.splice(0, a), (t.bufferedIndex = 0))
					: (t.bufferedIndex = a);
		}
		t.bufferProcessing = !1;
	}
	o(Xt, 'clearBuffer');
	I.prototype._write = function (e, t, n) {
		if (this._writev)
			this._writev([{ chunk: e, encoding: t }], n);
		else throw new Uf('_write()');
	};
	I.prototype._writev = null;
	I.prototype.end = function (e, t, n) {
		const r = this._writableState;
		typeof e == 'function'
			? ((n = e), (e = null), (t = null))
			: typeof t == 'function' && ((n = t), (t = null));
		let i;
		if (e != null) {
			const l = si(this, e, t);
			l instanceof Pf && (i = l);
		}
		return (
			r.corked && ((r.corked = 1), this.uncork()),
			i
			|| (!r.errored && !r.ending
				? ((r.ending = !0), Jt(this, r, !0), (r.ended = !0))
				: r.finished
					? (i = new Gf('end'))
					: r.destroyed && (i = new De('end'))),
			typeof n == 'function' && (i || r.finished ? ae.nextTick(n, i) : r[Ee].push(n)),
			this
		);
	};
	function tt(e) {
		return (
			e.ending
			&& !e.destroyed
			&& e.constructed
			&& e.length === 0
			&& !e.errored
			&& e.buffered.length === 0
			&& !e.finished
			&& !e.writing
			&& !e.errorEmitted
			&& !e.closeEmitted
		);
	}
	o(tt, 'needFinish');
	function Jf(e, t) {
		let n = !1;
		function r(i) {
			if (n) {
				Se(e, i ?? fi());
				return;
			}
			if (((n = !0), t.pendingcb--, i)) {
				const l = t[Ee].splice(0);
				for (let a = 0; a < l.length; a++) l[a](i);
				Se(e, i, t.sync);
			}
			else {
				tt(t) && ((t.prefinished = !0), e.emit('prefinish'), t.pendingcb++, ae.nextTick(Yt, e, t));
			}
		}
		(o(r, 'onFinish'), (t.sync = !0), t.pendingcb++);
		try {
			e._final(r);
		}
		catch (i) {
			r(i);
		}
		t.sync = !1;
	}
	o(Jf, 'callFinal');
	function Qf(e, t) {
		!t.prefinished
		&& !t.finalCalled
		&& (typeof e._final == 'function' && !t.destroyed
			? ((t.finalCalled = !0), Jf(e, t))
			: ((t.prefinished = !0), e.emit('prefinish')));
	}
	o(Qf, 'prefinish');
	function Jt(e, t, n) {
		tt(t)
		&& (Qf(e, t),
		t.pendingcb === 0
		&& (n
			? (t.pendingcb++,
				ae.nextTick(
					(r, i) => {
						tt(i) ? Yt(r, i) : i.pendingcb--;
					},
					e,
					t,
				))
			: tt(t) && (t.pendingcb++, Yt(e, t))));
	}
	o(Jt, 'finishMaybe');
	function Yt(e, t) {
		(t.pendingcb--, (t.finished = !0));
		const n = t[Ee].splice(0);
		for (let r = 0; r < n.length; r++) n[r]();
		if ((e.emit('finish'), t.autoDestroy)) {
			const r = e._readableState;
			(!r || (r.autoDestroy && (r.endEmitted || r.readable === !1))) && e.destroy();
		}
	}
	o(Yt, 'finish');
	Of(I.prototype, {
		closed: {
			__proto__: null,
			get() {
				return this._writableState ? this._writableState.closed : !1;
			},
		},
		destroyed: {
			__proto__: null,
			get() {
				return this._writableState ? this._writableState.destroyed : !1;
			},
			set(e) {
				this._writableState && (this._writableState.destroyed = e);
			},
		},
		writable: {
			__proto__: null,
			get() {
				const e = this._writableState;
				return !!e && e.writable !== !1 && !e.destroyed && !e.errored && !e.ending && !e.ended;
			},
			set(e) {
				this._writableState && (this._writableState.writable = !!e);
			},
		},
		writableFinished: {
			__proto__: null,
			get() {
				return this._writableState ? this._writableState.finished : !1;
			},
		},
		writableObjectMode: {
			__proto__: null,
			get() {
				return this._writableState ? this._writableState.objectMode : !1;
			},
		},
		writableBuffer: {
			__proto__: null,
			get() {
				return this._writableState && this._writableState.getBuffer();
			},
		},
		writableEnded: {
			__proto__: null,
			get() {
				return this._writableState ? this._writableState.ending : !1;
			},
		},
		writableNeedDrain: {
			__proto__: null,
			get() {
				const e = this._writableState;
				return e ? !e.destroyed && !e.ending && e.needDrain : !1;
			},
		},
		writableHighWaterMark: {
			__proto__: null,
			get() {
				return this._writableState && this._writableState.highWaterMark;
			},
		},
		writableCorked: {
			__proto__: null,
			get() {
				return this._writableState ? this._writableState.corked : 0;
			},
		},
		writableLength: {
			__proto__: null,
			get() {
				return this._writableState && this._writableState.length;
			},
		},
		errored: {
			__proto__: null,
			enumerable: !1,
			get() {
				return this._writableState ? this._writableState.errored : null;
			},
		},
		writableAborted: {
			__proto__: null,
			enumerable: !1,
			get: o(function () {
				return !!(
					this._writableState.writable !== !1
					&& (this._writableState.destroyed || this._writableState.errored)
					&& !this._writableState.finished
				);
			}, 'get'),
		},
	});
	const Zf = rt.destroy;
	I.prototype.destroy = function (e, t) {
		const n = this._writableState;
		return (
			!n.destroyed && (n.bufferedIndex < n.buffered.length || n[Ee].length) && ae.nextTick(zt, n),
			Zf.call(this, e, t),
			this
		);
	};
	I.prototype._undestroy = rt.undestroy;
	I.prototype._destroy = function (e, t) {
		t(e);
	};
	I.prototype[$f.captureRejectionSymbol] = function (e) {
		this.destroy(e);
	};
	let Vt;
	function ci() {
		return (Vt === void 0 && (Vt = {}), Vt);
	}
	o(ci, 'lazyWebStreams');
	I.fromWeb = function (e, t) {
		return ci().newStreamWritableFromWritableStream(e, t);
	};
	I.toWeb = function (e) {
		return ci().newWritableStreamFromStreamWritable(e);
	};
});
const ki = A((ad, Mi) => {
	const Qt = N('process');
	const eu = N('buffer');
	const {
		isReadable: tu,
		isWritable: nu,
		isIterable: bi,
		isNodeStream: ru,
		isReadableNodeStream: pi,
		isWritableNodeStream: _i,
		isDuplexNodeStream: iu,
		isReadableStream: yi,
		isWritableStream: wi,
	} = B();
	const gi = Y();
	const {
		AbortError: Ti,
		codes: { ERR_INVALID_ARG_TYPE: ou, ERR_INVALID_RETURN_VALUE: Si },
	} = L();
	const { destroyer: Ae } = oe();
	const lu = H();
	const Ii = Ne();
	const au = it();
	const { createDeferredPromise: Ei } = O();
	const Ri = Ct();
	const Ai = globalThis.Blob || eu.Blob;
	const fu = o(
		typeof Ai < 'u'
			? (t) => {
					return t instanceof Ai;
				}
			: (t) => {
					return !1;
				},
		'isBlob',
	);
	const uu = globalThis.AbortController || he().AbortController;
	const { FunctionPrototypeCall: mi } = T();
	const ee = class extends lu {
		static {
			o(this, 'Duplexify');
		}

		constructor(t) {
			(super(t),
			t?.readable === !1
			&& ((this._readableState.readable = !1),
			(this._readableState.ended = !0),
			(this._readableState.endEmitted = !0)),
			t?.writable === !1
			&& ((this._writableState.writable = !1),
			(this._writableState.ending = !0),
			(this._writableState.ended = !0),
			(this._writableState.finished = !0)));
		}
	};
	Mi.exports = o(function e(t, n) {
		if (iu(t))
			return t;
		if (pi(t))
			return Re({ readable: t });
		if (_i(t))
			return Re({ writable: t });
		if (ru(t))
			return Re({ writable: !1, readable: !1 });
		if (yi(t))
			return Re({ readable: Ii.fromWeb(t) });
		if (wi(t))
			return Re({ writable: au.fromWeb(t) });
		if (typeof t == 'function') {
			const { value: i, write: l, final: a, destroy: u } = su(t);
			if (bi(i))
				return Ri(ee, i, { objectMode: !0, write: l, final: a, destroy: u });
			const s = i?.then;
			if (typeof s == 'function') {
				let f;
				const d = mi(
					s,
					i,
					(c) => {
						if (c != null)
							throw new Si('nully', 'body', c);
					},
					(c) => {
						Ae(f, c);
					},
				);
				return (f = new ee({
					objectMode: !0,
					readable: !1,
					write: l,
					final(c) {
						a(async () => {
							try {
								(await d, Qt.nextTick(c, null));
							}
							catch (p) {
								Qt.nextTick(c, p);
							}
						});
					},
					destroy: u,
				}));
			}
			throw new Si('Iterable, AsyncIterable or AsyncFunction', n, i);
		}
		if (fu(t))
			return e(t.arrayBuffer());
		if (bi(t))
			return Ri(ee, t, { objectMode: !0, writable: !1 });
		if (yi(t?.readable) && wi(t?.writable))
			return ee.fromWeb(t);
		if (typeof t?.writable == 'object' || typeof t?.readable == 'object') {
			const i = t != null && t.readable ? (pi(t?.readable) ? t?.readable : e(t.readable)) : void 0;
			const l = t != null && t.writable ? (_i(t?.writable) ? t?.writable : e(t.writable)) : void 0;
			return Re({ readable: i, writable: l });
		}
		const r = t?.then;
		if (typeof r == 'function') {
			let i;
			return (
				mi(
					r,
					t,
					(l) => {
						(l != null && i.push(l), i.push(null));
					},
					(l) => {
						Ae(i, l);
					},
				),
				(i = new ee({ objectMode: !0, writable: !1, read() {} }))
			);
		}
		throw new ou(
			n,
			[
				'Blob',
				'ReadableStream',
				'WritableStream',
				'Stream',
				'Iterable',
				'AsyncIterable',
				'Function',
				'{ readable, writable } pair',
				'Promise',
			],
			t,
		);
	}, 'duplexify');
	function su(e) {
		let { promise: t, resolve: n } = Ei();
		const r = new uu();
		const i = r.signal;
		return {
			value: e(
				(async function* () {
					for (;;) {
						const a = t;
						t = null;
						const { chunk: u, done: s, cb: f } = await a;
						if ((Qt.nextTick(f), s))
							return;
						if (i.aborted)
							throw new Ti(void 0, { cause: i.reason });
						(({ promise: t, resolve: n } = Ei()), yield u);
					}
				})(),
				{ signal: i },
			),
			write(a, u, s) {
				const f = n;
				((n = null), f({ chunk: a, done: !1, cb: s }));
			},
			final(a) {
				const u = n;
				((n = null), u({ done: !0, cb: a }));
			},
			destroy(a, u) {
				(r.abort(), u(a));
			},
		};
	}
	o(su, 'fromAsyncGen');
	function Re(e) {
		const t = e.readable && typeof e.readable.read != 'function' ? Ii.wrap(e.readable) : e.readable;
		const n = e.writable;
		let r = !!tu(t);
		let i = !!nu(n);
		let l;
		let a;
		let u;
		let s;
		let f;
		function d(c) {
			const p = s;
			((s = null), p ? p(c) : c && f.destroy(c));
		}
		return (
			o(d, 'onfinished'),
			(f = new ee({
				readableObjectMode: !!(t != null && t.readableObjectMode),
				writableObjectMode: !!(n != null && n.writableObjectMode),
				readable: r,
				writable: i,
			})),
			i
			&& (gi(n, (c) => {
				((i = !1), c && Ae(t, c), d(c));
			}),
			(f._write = function (c, p, h) {
				n.write(c, p) ? h() : (l = h);
			}),
			(f._final = function (c) {
				(n.end(), (a = c));
			}),
			n.on('drain', () => {
				if (l) {
					const c = l;
					((l = null), c());
				}
			}),
			n.on('finish', () => {
				if (a) {
					const c = a;
					((a = null), c());
				}
			})),
			r
			&& (gi(t, (c) => {
				((r = !1), c && Ae(t, c), d(c));
			}),
			t.on('readable', () => {
				if (u) {
					const c = u;
					((u = null), c());
				}
			}),
			t.on('end', () => {
				f.push(null);
			}),
			(f._read = function () {
				for (;;) {
					const c = t.read();
					if (c === null) {
						u = f._read;
						return;
					}
					if (!f.push(c))
						return;
				}
			})),
			(f._destroy = function (c, p) {
				(!c && s !== null && (c = new Ti()),
				(u = null),
				(l = null),
				(a = null),
				s === null ? p(c) : ((s = p), Ae(n, c), Ae(t, c)));
			}),
			f
		);
	}
	o(Re, '_duplexify');
});
var H = A((ud, Di) => {
	'use strict';
	const {
		ObjectDefineProperties: du,
		ObjectGetOwnPropertyDescriptor: K,
		ObjectKeys: cu,
		ObjectSetPrototypeOf: Ni,
	} = T();
	Di.exports = F;
	const tn = Ne();
	const $ = it();
	Ni(F.prototype, tn.prototype);
	Ni(F, tn);
	{
		const e = cu($.prototype);
		for (let t = 0; t < e.length; t++) {
			const n = e[t];
			F.prototype[n] || (F.prototype[n] = $.prototype[n]);
		}
	}
	function F(e) {
		if (!(this instanceof F))
			return new F(e);
		(tn.call(this, e),
		$.call(this, e),
		e
			? ((this.allowHalfOpen = e.allowHalfOpen !== !1),
				e.readable === !1
				&& ((this._readableState.readable = !1),
				(this._readableState.ended = !0),
				(this._readableState.endEmitted = !0)),
				e.writable === !1
				&& ((this._writableState.writable = !1),
				(this._writableState.ending = !0),
				(this._writableState.ended = !0),
				(this._writableState.finished = !0)))
			: (this.allowHalfOpen = !0));
	}
	o(F, 'Duplex');
	du(F.prototype, {
		writable: { __proto__: null, ...K($.prototype, 'writable') },
		writableHighWaterMark: { __proto__: null, ...K($.prototype, 'writableHighWaterMark') },
		writableObjectMode: { __proto__: null, ...K($.prototype, 'writableObjectMode') },
		writableBuffer: { __proto__: null, ...K($.prototype, 'writableBuffer') },
		writableLength: { __proto__: null, ...K($.prototype, 'writableLength') },
		writableFinished: { __proto__: null, ...K($.prototype, 'writableFinished') },
		writableCorked: { __proto__: null, ...K($.prototype, 'writableCorked') },
		writableEnded: { __proto__: null, ...K($.prototype, 'writableEnded') },
		writableNeedDrain: { __proto__: null, ...K($.prototype, 'writableNeedDrain') },
		destroyed: {
			__proto__: null,
			get() {
				return this._readableState === void 0 || this._writableState === void 0
					? !1
					: this._readableState.destroyed && this._writableState.destroyed;
			},
			set(e) {
				this._readableState
				&& this._writableState
				&& ((this._readableState.destroyed = e), (this._writableState.destroyed = e));
			},
		},
	});
	let Zt;
	function qi() {
		return (Zt === void 0 && (Zt = {}), Zt);
	}
	o(qi, 'lazyWebStreams');
	F.fromWeb = function (e, t) {
		return qi().newStreamDuplexFromReadableWritablePair(e, t);
	};
	F.toWeb = function (e) {
		return qi().newReadableWritablePairFromDuplex(e);
	};
	let en;
	F.from = function (e) {
		return (en || (en = ki()), en(e, 'body'));
	};
});
const on = A((dd, Pi) => {
	'use strict';
	const { ObjectSetPrototypeOf: Li, Symbol: hu } = T();
	Pi.exports = z;
	const { ERR_METHOD_NOT_IMPLEMENTED: bu } = L().codes;
	const rn = H();
	const { getHighWaterMark: pu } = Me();
	Li(z.prototype, rn.prototype);
	Li(z, rn);
	const Pe = hu('kCallback');
	function z(e) {
		if (!(this instanceof z))
			return new z(e);
		const t = e ? pu(this, e, 'readableHighWaterMark', !0) : null;
		(t === 0
			&& (e = {
				...e,
				highWaterMark: null,
				readableHighWaterMark: t,
				writableHighWaterMark: e.writableHighWaterMark || 0,
			}),
		rn.call(this, e),
		(this._readableState.sync = !1),
		(this[Pe] = null),
		e
		&& (typeof e.transform == 'function' && (this._transform = e.transform),
		typeof e.flush == 'function' && (this._flush = e.flush)),
		this.on('prefinish', _u));
	}
	o(z, 'Transform');
	function nn(e) {
		typeof this._flush == 'function' && !this.destroyed
			? this._flush((t, n) => {
					if (t) {
						e ? e(t) : this.destroy(t);
						return;
					}
					(n != null && this.push(n), this.push(null), e && e());
				})
			: (this.push(null), e && e());
	}
	o(nn, 'final');
	function _u() {
		this._final !== nn && nn.call(this);
	}
	o(_u, 'prefinish');
	z.prototype._final = nn;
	z.prototype._transform = function (e, t, n) {
		throw new bu('_transform()');
	};
	z.prototype._write = function (e, t, n) {
		const r = this._readableState;
		const i = this._writableState;
		const l = r.length;
		this._transform(e, t, (a, u) => {
			if (a) {
				n(a);
				return;
			}
			(u != null && this.push(u),
			i.ended || l === r.length || r.length < r.highWaterMark ? n() : (this[Pe] = n));
		});
	};
	z.prototype._read = function () {
		if (this[Pe]) {
			const e = this[Pe];
			((this[Pe] = null), e());
		}
	};
});
const an = A((hd, Wi) => {
	'use strict';
	const { ObjectSetPrototypeOf: Oi } = T();
	Wi.exports = me;
	const ln = on();
	Oi(me.prototype, ln.prototype);
	Oi(me, ln);
	function me(e) {
		if (!(this instanceof me))
			return new me(e);
		ln.call(this, e);
	}
	o(me, 'PassThrough');
	me.prototype._transform = function (e, t, n) {
		n(null, e);
	};
});
const ft = A((pd, Fi) => {
	const Oe = N('process');
	const { ArrayIsArray: yu, Promise: wu, SymbolAsyncIterator: gu, SymbolDispose: Su } = T();
	const at = Y();
	const { once: Eu } = O();
	const Ru = oe();
	const xi = H();
	const {
		aggregateTwoErrors: Au,
		codes: {
			ERR_INVALID_ARG_TYPE: _n,
			ERR_INVALID_RETURN_VALUE: fn,
			ERR_MISSING_ARGS: mu,
			ERR_STREAM_DESTROYED: Tu,
			ERR_STREAM_PREMATURE_CLOSE: Iu,
		},
		AbortError: Mu,
	} = L();
	const { validateFunction: ku, validateAbortSignal: Nu } = pe();
	const {
		isIterable: fe,
		isReadable: un,
		isReadableNodeStream: lt,
		isNodeStream: Ci,
		isTransformStream: Te,
		isWebStream: qu,
		isReadableStream: sn,
		isReadableFinished: Du,
	} = B();
	const Lu = globalThis.AbortController || he().AbortController;
	let dn;
	let cn;
	let hn;
	function $i(e, t, n) {
		let r = !1;
		e.on('close', () => {
			r = !0;
		});
		const i = at(e, { readable: t, writable: n }, (l) => {
			r = !l;
		});
		return {
			destroy: o((l) => {
				r || ((r = !0), Ru.destroyer(e, l || new Tu('pipe')));
			}, 'destroy'),
			cleanup: i,
		};
	}
	o($i, 'destroyer');
	function Pu(e) {
		return (ku(e.at(-1), 'streams[stream.length - 1]'), e.pop());
	}
	o(Pu, 'popCallback');
	function bn(e) {
		if (fe(e))
			return e;
		if (lt(e))
			return Ou(e);
		throw new _n('val', ['Readable', 'Iterable', 'AsyncIterable'], e);
	}
	o(bn, 'makeAsyncIterable');
	async function* Ou(e) {
		(cn || (cn = Ne()), yield* cn.prototype[gu].call(e));
	}
	o(Ou, 'fromReadable');
	async function ot(e, t, n, { end: r }) {
		let i;
		let l = null;
		const a = o((f) => {
			if ((f && (i = f), l)) {
				const d = l;
				((l = null), d());
			}
		}, 'resume');
		const u = o(
			() =>
				new wu((f, d) => {
					i
						? d(i)
						: (l = o(() => {
								i ? d(i) : f();
							}, 'onresolve'));
				}),
			'wait',
		);
		t.on('drain', a);
		const s = at(t, { readable: !1 }, a);
		try {
			t.writableNeedDrain && (await u());
			for await (const f of e) t.write(f) || (await u());
			(r && (t.end(), await u()), n());
		}
		catch (f) {
			n(i !== f ? Au(i, f) : f);
		}
		finally {
			(s(), t.off('drain', a));
		}
	}
	o(ot, 'pumpToNode');
	async function pn(e, t, n, { end: r }) {
		Te(t) && (t = t.writable);
		const i = t.getWriter();
		try {
			for await (const l of e) (await i.ready, i.write(l).catch(() => {}));
			(await i.ready, r && (await i.close()), n());
		}
		catch (l) {
			try {
				(await i.abort(l), n(l));
			}
			catch (a) {
				n(a);
			}
		}
	}
	o(pn, 'pumpToWeb');
	function Wu(...e) {
		return ji(e, Eu(Pu(e)));
	}
	o(Wu, 'pipeline');
	function ji(e, t, n) {
		if ((e.length === 1 && yu(e[0]) && (e = e[0]), e.length < 2))
			throw new mu('streams');
		const r = new Lu();
		const i = r.signal;
		const l = n?.signal;
		const a = [];
		Nu(l, 'options.signal');
		function u() {
			S(new Mu());
		}
		(o(u, 'abort'), (hn = hn || O().addAbortListener));
		let s;
		l && (s = hn(l, u));
		let f;
		let d;
		const c = [];
		let p = 0;
		function h(k) {
			S(k, --p === 0);
		}
		o(h, 'finish');
		function S(k, g) {
			let M;
			if ((k && (!f || f.code === 'ERR_STREAM_PREMATURE_CLOSE') && (f = k), !(!f && !g))) {
				for (; c.length;) c.shift()(f);
				((M = s) === null || M === void 0 || M[Su](),
				r.abort(),
				g && (f || a.forEach(te => te()), Oe.nextTick(t, f, d)));
			}
		}
		o(S, 'finishImpl');
		let b;
		for (let k = 0; k < e.length; k++) {
			const g = e[k];
			const M = k < e.length - 1;
			const te = k > 0;
			const x = M || n?.end !== !1;
			const ce = k === e.length - 1;
			if (Ci(g)) {
				const W = function (V) {
					V && V.name !== 'AbortError' && V.code !== 'ERR_STREAM_PREMATURE_CLOSE' && h(V);
				};
				const R = W;
				if ((o(W, 'onError'), x)) {
					const { destroy: V, cleanup: ht } = $i(g, M, te);
					(c.push(V), un(g) && ce && a.push(ht));
				}
				(g.on('error', W),
				un(g)
				&& ce
				&& a.push(() => {
					g.removeListener('error', W);
				}));
			}
			if (k === 0) {
				if (typeof g == 'function') {
					if (((b = g({ signal: i })), !fe(b)))
						throw new fn('Iterable, AsyncIterable or Stream', 'source', b);
				}
				else {
					fe(g) || lt(g) || Te(g) ? (b = g) : (b = xi.from(g));
				}
			}
			else if (typeof g == 'function') {
				if (Te(b)) {
					var E;
					b = bn((E = b) === null || E === void 0 ? void 0 : E.readable);
				}
				else {
					b = bn(b);
				}
				if (((b = g(b, { signal: i })), M)) {
					if (!fe(b, !0))
						throw new fn('AsyncIterable', `transform[${k - 1}]`, b);
				}
				else {
					var y;
					dn || (dn = an());
					const W = new dn({ objectMode: !0 });
					const V = (y = b) === null || y === void 0 ? void 0 : y.then;
					if (typeof V == 'function') {
						(p++,
						V.call(
							b,
							(J) => {
								((d = J), J != null && W.write(J), x && W.end(), Oe.nextTick(h));
							},
							(J) => {
								(W.destroy(J), Oe.nextTick(h, J));
							},
						));
					}
					else if (fe(b, !0)) {
						(p++, ot(b, W, h, { end: x }));
					}
					else if (sn(b) || Te(b)) {
						const J = b.readable || b;
						(p++, ot(J, W, h, { end: x }));
					}
					else {
						throw new fn('AsyncIterable or Promise', 'destination', b);
					}
					b = W;
					const { destroy: ht, cleanup: _o } = $i(b, !1, !0);
					(c.push(ht), ce && a.push(_o));
				}
			}
			else if (Ci(g)) {
				if (lt(b)) {
					p += 2;
					const W = xu(b, g, h, { end: x });
					un(g) && ce && a.push(W);
				}
				else if (Te(b) || sn(b)) {
					const W = b.readable || b;
					(p++, ot(W, g, h, { end: x }));
				}
				else if (fe(b)) {
					(p++, ot(b, g, h, { end: x }));
				}
				else {
					throw new _n(
						'val',
						['Readable', 'Iterable', 'AsyncIterable', 'ReadableStream', 'TransformStream'],
						b,
					);
				}
				b = g;
			}
			else if (qu(g)) {
				if (lt(b)) {
					(p++, pn(bn(b), g, h, { end: x }));
				}
				else if (sn(b) || fe(b)) {
					(p++, pn(b, g, h, { end: x }));
				}
				else if (Te(b)) {
					(p++, pn(b.readable, g, h, { end: x }));
				}
				else {
					throw new _n(
						'val',
						['Readable', 'Iterable', 'AsyncIterable', 'ReadableStream', 'TransformStream'],
						b,
					);
				}
				b = g;
			}
			else {
				b = xi.from(g);
			}
		}
		return (((i != null && i.aborted) || (l != null && l.aborted)) && Oe.nextTick(u), b);
	}
	o(ji, 'pipelineImpl');
	function xu(e, t, n, { end: r }) {
		let i = !1;
		if (
			(t.on('close', () => {
				i || n(new Iu());
			}),
			e.pipe(t, { end: !1 }),
			r)) {
			const a = function () {
				((i = !0), t.end());
			};
			const l = a;
			(o(a, 'endFn'), Du(e) ? Oe.nextTick(a) : e.once('end', a));
		}
		else {
			n();
		}
		return (
			at(e, { readable: !0, writable: !1 }, (a) => {
				const u = e._readableState;
				a
				&& a.code === 'ERR_STREAM_PREMATURE_CLOSE'
				&& u
				&& u.ended
				&& !u.errored
				&& !u.errorEmitted
					? e.once('end', n).once('error', n)
					: n(a);
			}),
			at(t, { readable: !1, writable: !0 }, n)
		);
	}
	o(xu, 'pipe');
	Fi.exports = { pipelineImpl: ji, pipeline: Wu };
});
const wn = A((yd, Vi) => {
	'use strict';
	const { pipeline: Cu } = ft();
	const ut = H();
	const { destroyer: $u } = oe();
	const {
		isNodeStream: st,
		isReadable: vi,
		isWritable: Bi,
		isWebStream: yn,
		isTransformStream: ue,
		isWritableStream: Ui,
		isReadableStream: Hi,
	} = B();
	const {
		AbortError: ju,
		codes: { ERR_INVALID_ARG_VALUE: Gi, ERR_MISSING_ARGS: Fu },
	} = L();
	const vu = Y();
	Vi.exports = o((...t) => {
		if (t.length === 0)
			throw new Fu('streams');
		if (t.length === 1)
			return ut.from(t[0]);
		const n = [...t];
		if (
			(typeof t[0] == 'function' && (t[0] = ut.from(t[0])), typeof t.at(-1) == 'function')
		) {
			const h = t.length - 1;
			t[h] = ut.from(t[h]);
		}
		for (let h = 0; h < t.length; ++h) {
			if (!(!st(t[h]) && !yn(t[h]))) {
				if (h < t.length - 1 && !(vi(t[h]) || Hi(t[h]) || ue(t[h])))
					throw new Gi(`streams[${h}]`, n[h], 'must be readable');
				if (h > 0 && !(Bi(t[h]) || Ui(t[h]) || ue(t[h])))
					throw new Gi(`streams[${h}]`, n[h], 'must be writable');
			}
		}
		let r, i, l, a, u;
		function s(h) {
			const S = a;
			((a = null), S ? S(h) : h ? u.destroy(h) : !p && !c && u.destroy());
		}
		o(s, 'onfinished');
		const f = t[0];
		const d = Cu(t, s);
		let c = !!(Bi(f) || Ui(f) || ue(f));
		let p = !!(vi(d) || Hi(d) || ue(d));
		if (
			((u = new ut({
				writableObjectMode: !!(f != null && f.writableObjectMode),
				readableObjectMode: !!(d != null && d.readableObjectMode),
				writable: c,
				readable: p,
			})),
			c)) {
			if (st(f)) {
				((u._write = function (S, b, E) {
					f.write(S, b) ? E() : (r = E);
				}),
				(u._final = function (S) {
					(f.end(), (i = S));
				}),
				f.on('drain', () => {
					if (r) {
						const S = r;
						((r = null), S());
					}
				}));
			}
			else if (yn(f)) {
				const b = (ue(f) ? f.writable : f).getWriter();
				((u._write = async function (E, y, R) {
					try {
						(await b.ready, b.write(E).catch(() => {}), R());
					}
					catch (k) {
						R(k);
					}
				}),
				(u._final = async function (E) {
					try {
						(await b.ready, b.close().catch(() => {}), (i = E));
					}
					catch (y) {
						E(y);
					}
				}));
			}
			const h = ue(d) ? d.readable : d;
			vu(h, () => {
				if (i) {
					const S = i;
					((i = null), S());
				}
			});
		}
		if (p) {
			if (st(d)) {
				(d.on('readable', () => {
					if (l) {
						const h = l;
						((l = null), h());
					}
				}),
				d.on('end', () => {
					u.push(null);
				}),
				(u._read = function () {
					for (;;) {
						const h = d.read();
						if (h === null) {
							l = u._read;
							return;
						}
						if (!u.push(h))
							return;
					}
				}));
			}
			else if (yn(d)) {
				const S = (ue(d) ? d.readable : d).getReader();
				u._read = async function () {
					for (;;) {
						try {
							const { value: b, done: E } = await S.read();
							if (!u.push(b))
								return;
							if (E) {
								u.push(null);
								return;
							}
						}
						catch {
							return;
						}
					}
				};
			}
		}
		return (
			(u._destroy = function (h, S) {
				(!h && a !== null && (h = new ju()),
				(l = null),
				(r = null),
				(i = null),
				a === null ? S(h) : ((a = S), st(d) && $u(d, h)));
			}),
			u
		);
	}, 'compose');
});
const no = A((gd, En) => {
	'use strict';
	const Bu = globalThis.AbortController || he().AbortController;
	const {
		codes: {
			ERR_INVALID_ARG_VALUE: Uu,
			ERR_INVALID_ARG_TYPE: We,
			ERR_MISSING_ARGS: Hu,
			ERR_OUT_OF_RANGE: Gu,
		},
		AbortError: G,
	} = L();
	const { validateAbortSignal: se, validateInteger: Yi, validateObject: de } = pe();
	const Vu = T().Symbol('kWeak');
	const Yu = T().Symbol('kResistStopPropagation');
	const { finished: Ku } = Y();
	const zu = wn();
	const { addAbortSignalNoValidate: Xu } = Ie();
	const { isWritable: Ju, isNodeStream: Qu } = B();
	const { deprecate: Zu } = O();
	const {
		ArrayPrototypePush: es,
		Boolean: ts,
		MathFloor: Ki,
		Number: ns,
		NumberIsNaN: rs,
		Promise: zi,
		PromiseReject: Xi,
		PromiseResolve: is,
		PromisePrototypeThen: Ji,
		Symbol: Zi,
	} = T();
	const dt = Zi('kEmpty');
	const Qi = Zi('kEof');
	function os(e, t) {
		if (
			(t != null && de(t, 'options'),
			t?.signal != null && se(t.signal, 'options.signal'),
			Qu(e) && !Ju(e))) {
			throw new Uu('stream', e, 'must be writable');
		}
		const n = zu(this, e);
		return (t != null && t.signal && Xu(t.signal, n), n);
	}
	o(os, 'compose');
	function ct(e, t) {
		if (typeof e != 'function')
			throw new We('fn', ['Function', 'AsyncFunction'], e);
		(t != null && de(t, 'options'), t?.signal != null && se(t.signal, 'options.signal'));
		let n = 1;
		t?.concurrency != null && (n = Ki(t.concurrency));
		let r = n - 1;
		return (
			t?.highWaterMark != null && (r = Ki(t.highWaterMark)),
			Yi(n, 'options.concurrency', 1),
			Yi(r, 'options.highWaterMark', 0),
			(r += n),
			o(async function* () {
				const l = O().AbortSignalAny([t?.signal].filter(ts));
				const a = this;
				const u = [];
				const s = { signal: l };
				let f;
				let d;
				let c = !1;
				let p = 0;
				function h() {
					((c = !0), S());
				}
				o(h, 'onCatch');
				function S() {
					((p -= 1), b());
				}
				o(S, 'afterItemProcessed');
				function b() {
					d && !c && p < n && u.length < r && (d(), (d = null));
				}
				o(b, 'maybeResume');
				async function E() {
					try {
						for await (let y of a) {
							if (c)
								return;
							if (l.aborted)
								throw new G();
							try {
								if (((y = e(y, s)), y === dt))
									continue;
								y = is(y);
							}
							catch (R) {
								y = Xi(R);
							}
							((p += 1),
							Ji(y, S, h),
							u.push(y),
							f && (f(), (f = null)),
							!c
							&& (u.length >= r || p >= n)
							&& (await new zi((R) => {
								d = R;
							})));
						}
						u.push(Qi);
					}
					catch (y) {
						const R = Xi(y);
						(Ji(R, S, h), u.push(R));
					}
					finally {
						((c = !0), f && (f(), (f = null)));
					}
				}
				(o(E, 'pump'), E());
				try {
					for (;;) {
						for (; u.length > 0;) {
							const y = await u[0];
							if (y === Qi)
								return;
							if (l.aborted)
								throw new G();
							(y !== dt && (yield y), u.shift(), b());
						}
						await new zi((y) => {
							f = y;
						});
					}
				}
				finally {
					((c = !0), d && (d(), (d = null)));
				}
			}, 'map').call(this)
		);
	}
	o(ct, 'map');
	function ls(e = void 0) {
		return (
			e != null && de(e, 'options'),
			e?.signal != null && se(e.signal, 'options.signal'),
			o(async function* () {
				let n = 0;
				for await (const i of this) {
					var r;
					if (e != null && (r = e.signal) !== null && r !== void 0 && r.aborted)
						throw new G({ cause: e.signal.reason });
					yield [n++, i];
				}
			}, 'asIndexedPairs').call(this)
		);
	}
	o(ls, 'asIndexedPairs');
	async function eo(e, t = void 0) {
		for await (const n of Sn.call(this, e, t)) return !0;
		return !1;
	}
	o(eo, 'some');
	async function as(e, t = void 0) {
		if (typeof e != 'function')
			throw new We('fn', ['Function', 'AsyncFunction'], e);
		return !(await eo.call(this, async (...n) => !(await e(...n)), t));
	}
	o(as, 'every');
	async function fs(e, t) {
		for await (const n of Sn.call(this, e, t)) return n;
	}
	o(fs, 'find');
	async function us(e, t) {
		if (typeof e != 'function')
			throw new We('fn', ['Function', 'AsyncFunction'], e);
		async function n(r, i) {
			return (await e(r, i), dt);
		}
		o(n, 'forEachFn');
		for await (const r of ct.call(this, n, t));
	}
	o(us, 'forEach');
	function Sn(e, t) {
		if (typeof e != 'function')
			throw new We('fn', ['Function', 'AsyncFunction'], e);
		async function n(r, i) {
			return (await e(r, i)) ? r : dt;
		}
		return (o(n, 'filterFn'), ct.call(this, n, t));
	}
	o(Sn, 'filter');
	const gn = class extends Hu {
		static {
			o(this, 'ReduceAwareErrMissingArgs');
		}

		constructor() {
			(super('reduce'), (this.message = 'Reduce of an empty stream requires an initial value'));
		}
	};
	async function ss(e, t, n) {
		let r;
		if (typeof e != 'function')
			throw new We('reducer', ['Function', 'AsyncFunction'], e);
		(n != null && de(n, 'options'), n?.signal != null && se(n.signal, 'options.signal'));
		let i = arguments.length > 1;
		if (n != null && (r = n.signal) !== null && r !== void 0 && r.aborted) {
			const f = new G(void 0, { cause: n.signal.reason });
			throw (this.once('error', () => {}), await Ku(this.destroy(f)), f);
		}
		const l = new Bu();
		const a = l.signal;
		if (n != null && n.signal) {
			const f = { once: !0, [Vu]: this, [Yu]: !0 };
			n.signal.addEventListener('abort', () => l.abort(), f);
		}
		let u = !1;
		try {
			for await (const f of this) {
				var s;
				if (((u = !0), n != null && (s = n.signal) !== null && s !== void 0 && s.aborted))
					throw new G();
				i ? (t = await e(t, f, { signal: a })) : ((t = f), (i = !0));
			}
			if (!u && !i)
				throw new gn();
		}
		finally {
			l.abort();
		}
		return t;
	}
	o(ss, 'reduce');
	async function ds(e) {
		(e != null && de(e, 'options'), e?.signal != null && se(e.signal, 'options.signal'));
		const t = [];
		for await (const r of this) {
			var n;
			if (e != null && (n = e.signal) !== null && n !== void 0 && n.aborted)
				throw new G(void 0, { cause: e.signal.reason });
			es(t, r);
		}
		return t;
	}
	o(ds, 'toArray');
	function cs(e, t) {
		const n = ct.call(this, e, t);
		return o(async function* () {
			for await (const i of n) yield* i;
		}, 'flatMap').call(this);
	}
	o(cs, 'flatMap');
	function to(e) {
		if (((e = ns(e)), rs(e)))
			return 0;
		if (e < 0)
			throw new Gu('number', '>= 0', e);
		return e;
	}
	o(to, 'toIntegerOrInfinity');
	function hs(e, t = void 0) {
		return (
			t != null && de(t, 'options'),
			t?.signal != null && se(t.signal, 'options.signal'),
			(e = to(e)),
			o(async function* () {
				let r;
				if (t != null && (r = t.signal) !== null && r !== void 0 && r.aborted)
					throw new G();
				for await (const l of this) {
					var i;
					if (t != null && (i = t.signal) !== null && i !== void 0 && i.aborted)
						throw new G();
					e-- <= 0 && (yield l);
				}
			}, 'drop').call(this)
		);
	}
	o(hs, 'drop');
	function bs(e, t = void 0) {
		return (
			t != null && de(t, 'options'),
			t?.signal != null && se(t.signal, 'options.signal'),
			(e = to(e)),
			o(async function* () {
				let r;
				if (t != null && (r = t.signal) !== null && r !== void 0 && r.aborted)
					throw new G();
				for await (const l of this) {
					var i;
					if (t != null && (i = t.signal) !== null && i !== void 0 && i.aborted)
						throw new G();
					if ((e-- > 0 && (yield l), e <= 0))
						return;
				}
			}, 'take').call(this)
		);
	}
	o(bs, 'take');
	En.exports.streamReturningOperators = {
		asIndexedPairs: Zu(ls, 'readable.asIndexedPairs will be removed in a future version.'),
		drop: hs,
		filter: Sn,
		flatMap: cs,
		map: ct,
		take: bs,
		compose: os,
	};
	En.exports.promiseReturningOperators = {
		every: as,
		forEach: us,
		reduce: ss,
		toArray: ds,
		some: eo,
		find: fs,
	};
});
const ho = A((Ed, co) => {
	'use strict';
	const { Buffer: ps } = N('buffer');
	const { ObjectDefineProperty: X, ObjectKeys: oo, ReflectApply: lo } = T();
	const {
		promisify: { custom: ao },
	} = O();
	const { streamReturningOperators: ro, promiseReturningOperators: io } = no();
	const {
		codes: { ERR_ILLEGAL_CONSTRUCTOR: fo },
	} = L();
	const _s = wn();
	const { setDefaultHighWaterMark: ys, getDefaultHighWaterMark: ws } = Me();
	const { pipeline: uo } = ft();
	const { destroyer: gs } = oe();
	const so = Y();
	const Rn = An();
	const xe = B();
	const m = (co.exports = Ve().Stream);
	m.isDestroyed = xe.isDestroyed;
	m.isDisturbed = xe.isDisturbed;
	m.isErrored = xe.isErrored;
	m.isReadable = xe.isReadable;
	m.isWritable = xe.isWritable;
	m.Readable = Ne();
	for (const e of oo(ro)) {
		const n = function (...r) {
			if (new.target)
				throw fo();
			return m.Readable.from(lo(t, this, r));
		};
		o(n, 'fn');
		let t = ro[e];
		(X(n, 'name', { __proto__: null, value: t.name }),
		X(n, 'length', { __proto__: null, value: t.length }),
		X(m.Readable.prototype, e, {
			__proto__: null,
			value: n,
			enumerable: !1,
			configurable: !0,
			writable: !0,
		}));
	}
	for (const e of oo(io)) {
		const n = function (...r) {
			if (new.target)
				throw fo();
			return lo(t, this, r);
		};
		o(n, 'fn');
		let t = io[e];
		(X(n, 'name', { __proto__: null, value: t.name }),
		X(n, 'length', { __proto__: null, value: t.length }),
		X(m.Readable.prototype, e, {
			__proto__: null,
			value: n,
			enumerable: !1,
			configurable: !0,
			writable: !0,
		}));
	}
	m.Writable = it();
	m.Duplex = H();
	m.Transform = on();
	m.PassThrough = an();
	m.pipeline = uo;
	const { addAbortSignal: Ss } = Ie();
	m.addAbortSignal = Ss;
	m.finished = so;
	m.destroy = gs;
	m.compose = _s;
	m.setDefaultHighWaterMark = ys;
	m.getDefaultHighWaterMark = ws;
	X(m, 'promises', {
		__proto__: null,
		configurable: !0,
		enumerable: !0,
		get() {
			return Rn;
		},
	});
	X(uo, ao, {
		__proto__: null,
		enumerable: !0,
		get() {
			return Rn.pipeline;
		},
	});
	X(so, ao, {
		__proto__: null,
		enumerable: !0,
		get() {
			return Rn.finished;
		},
	});
	m.Stream = m;
	m._isUint8Array = o((t) => {
		return t instanceof Uint8Array;
	}, 'isUint8Array');
	m._uint8ArrayToBuffer = o((t) => {
		return ps.from(t.buffer, t.byteOffset, t.byteLength);
	}, '_uint8ArrayToBuffer');
});
var An = A((Ad, bo) => {
	'use strict';
	const { ArrayPrototypePop: Es, Promise: Rs } = T();
	const { isIterable: As, isNodeStream: ms, isWebStream: Ts } = B();
	const { pipelineImpl: Is } = ft();
	const { finished: Ms } = Y();
	ho();
	function ks(...e) {
		return new Rs((t, n) => {
			let r;
			let i;
			const l = e.at(-1);
			if (l && typeof l == 'object' && !ms(l) && !As(l) && !Ts(l)) {
				const a = Es(e);
				((r = a.signal), (i = a.end));
			}
			Is(
				e,
				(a, u) => {
					a ? n(a) : t(u);
				},
				{ signal: r, end: i },
			);
		});
	}
	o(ks, 'pipeline');
	bo.exports = { finished: Ms, pipeline: ks };
});
const po = Ao(An(), 1);
const export_finished = po.finished;
const export_pipeline = po.pipeline;
export { export_finished as finished, export_pipeline as pipeline };
