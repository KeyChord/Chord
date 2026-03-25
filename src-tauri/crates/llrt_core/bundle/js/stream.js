import { EventEmitter as Mo } from 'node:events';
import { inherits as Ys } from 'node:util';

const xo = Object.create;
const yt = Object.defineProperty;
const Co = Object.getOwnPropertyDescriptor;
const vo = Object.getOwnPropertyNames;
const $o = Object.getPrototypeOf;
const jo = Object.prototype.hasOwnProperty;
const o = (e, t) => yt(e, 'name', { value: t, configurable: !0 });
const k = (e =>
	typeof require < 'u'
		? require
		: typeof Proxy < 'u'
			? new Proxy(e, { get: (t, n) => (typeof require < 'u' ? require : t)[n] })
			: e)(function (e) {
	if (typeof require < 'u')
		return require.apply(this, arguments);
	throw new Error(`Dynamic require of "${e}" is not supported`);
});
const E = (e, t) => () => (t || e((t = { exports: {} }).exports, t), t.exports);
function Fo(e, t, n, r) {
	if ((t && typeof t == 'object') || typeof t == 'function') {
		for (const i of vo(t)) {
			!jo.call(e, i)
			&& i !== n
			&& yt(e, i, { get: () => t[i], enumerable: !(r = Co(t, i)) || r.enumerable });
		}
	}
	return e;
}
function re(e, t, n) {
	return (n = e != null ? xo($o(e)) : {}),
	Fo(t || !e || !e.__esModule ? yt(n, 'default', { value: e, enumerable: !0 }) : n, e);
}
const T = E((zs, In) => {
	'use strict';
	const wt = class extends Error {
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
	In.exports = {
		AggregateError: wt,
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
const gt = E((Js, Mn) => {
	'use strict';
	Mn.exports = {
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
const L = E((Qs, Nn) => {
	'use strict';
	const { format: Bo, inspect: Fe } = gt();
	const { AggregateError: Uo } = T();
	const Ho = globalThis.AggregateError || Uo;
	const Go = Symbol('kIsNodeError');
	const Vo = [
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
	const Yo = /^([A-Z][a-z0-9]*)+$/;
	const Ko = '__node_internal_';
	const Be = {};
	function ie(e, t) {
		if (!e)
			throw new Be.ERR_INTERNAL_ASSERTION(t);
	}
	o(ie, 'assert');
	function qn(e) {
		let t = '';
		let n = e.length;
		const r = e[0] === '-' ? 1 : 0;
		for (; n >= r + 4; n -= 3) t = `_${e.slice(n - 3, n)}${t}`;
		return `${e.slice(0, n)}${t}`;
	}
	o(qn, 'addNumericalSeparator');
	function zo(e, t, n) {
		if (typeof t == 'function') {
			return (
				ie(
					t.length <= n.length,
					`Code: ${e}; The provided arguments length (${n.length}) does not match the required ones (${t.length}).`,
				),
				t(...n)
			);
		}
		const r = (t.match(/%[dfijoOs]/g) || []).length;
		return (
			ie(
				r === n.length,
				`Code: ${e}; The provided arguments length (${n.length}) does not match the required ones (${r}).`,
			),
			n.length === 0 ? t : Bo(t, ...n)
		);
	}
	o(zo, 'getMessage');
	function D(e, t, n) {
		n || (n = Error);
		class r extends n {
			static {
				o(this, 'NodeError');
			}

			constructor(...l) {
				super(zo(e, t, l));
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
		(r.prototype[Go] = !0),
		(Be[e] = r));
	}
	o(D, 'E');
	function kn(e) {
		const t = Ko + e.name;
		return (Object.defineProperty(e, 'name', { value: t }), e);
	}
	o(kn, 'hideStackFrames');
	function Xo(e, t) {
		if (e && t && e !== t) {
			if (Array.isArray(t.errors))
				return (t.errors.push(e), t);
			const n = new Ho([t, e], t.message);
			return ((n.code = t.code), n);
		}
		return e || t;
	}
	o(Xo, 'aggregateTwoErrors');
	const St = class extends Error {
		static {
			o(this, 'AbortError');
		}

		constructor(t = 'The operation was aborted', n = void 0) {
			if (n !== void 0 && typeof n != 'object')
				throw new Be.ERR_INVALID_ARG_TYPE('options', 'Object', n);
			(super(t, n), (this.code = 'ABORT_ERR'), (this.name = 'AbortError'));
		}
	};
	D('ERR_ASSERTION', '%s', Error);
	D(
		'ERR_INVALID_ARG_TYPE',
		(e, t, n) => {
			(ie(typeof e == 'string', '\'name\' must be a string'), Array.isArray(t) || (t = [t]));
			let r = 'The ';
			(e.endsWith(' argument')
				? (r += `${e} `)
				: (r += `"${e}" ${e.includes('.') ? 'property' : 'argument'} `),
			(r += 'must be '));
			const i = [];
			const l = [];
			const a = [];
			for (const s of t) {
				(ie(typeof s == 'string', 'All expected entries have to be of type string'),
				Vo.includes(s)
					? i.push(s.toLowerCase())
					: Yo.test(s)
						? l.push(s)
						: (ie(s !== 'object', 'The value "object" should be written as "Object"'),
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
			ie(e.length > 0, 'At least one arg needs to be specified');
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
			ie(t, 'Missing "range" argument');
			let r;
			if (Number.isInteger(n) && Math.abs(n) > 2 ** 32) {
				r = qn(String(n));
			}
			else if (typeof n == 'bigint') {
				r = String(n);
				const i = BigInt(2) ** BigInt(32);
				((n > i || n < -i) && (r = qn(r)), (r += 'n'));
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
	Nn.exports = { AbortError: St, aggregateTwoErrors: kn(Xo), hideStackFrames: kn, codes: Be };
});
const _e = E((ed, Ue) => {
	'use strict';
	const { AbortController: Dn, AbortSignal: Jo }
		= typeof self < 'u' ? self : typeof window < 'u' ? window : void 0;
	Ue.exports = Dn;
	Ue.exports.AbortSignal = Jo;
	Ue.exports.default = Dn;
});
const O = E((td, Rt) => {
	'use strict';
	const Qo = k('buffer');
	const { format: Zo, inspect: el } = gt();
	const {
		codes: { ERR_INVALID_ARG_TYPE: Et },
	} = L();
	const { kResistStopPropagation: tl, AggregateError: nl, SymbolDispose: rl } = T();
	const il = globalThis.AbortSignal || _e().AbortSignal;
	const ol = globalThis.AbortController || _e().AbortController;
	const ll = Object.getPrototypeOf(async () => {}).constructor;
	const Ln = globalThis.Blob || Qo.Blob;
	const al = o(
		typeof Ln < 'u'
			? (t) => {
					return t instanceof Ln;
				}
			: (t) => {
					return !1;
				},
		'isBlob',
	);
	const Pn = o((e, t) => {
		if (e !== void 0 && (e === null || typeof e != 'object' || !('aborted' in e)))
			throw new Et(t, 'AbortSignal', e);
	}, 'validateAbortSignal');
	const fl = o((e, t) => {
		if (typeof e != 'function')
			throw new Et(t, 'Function', e);
	}, 'validateFunction');
	Rt.exports = {
		AggregateError: nl,
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
		format: Zo,
		inspect: el,
		types: {
			isAsyncFunction(e) {
				return e instanceof ll;
			},
			isArrayBufferView(e) {
				return ArrayBuffer.isView(e);
			},
		},
		isBlob: al,
		deprecate(e, t) {
			return e;
		},
		addAbortListener:
      k('events').addAbortListener
      || o((t, n) => {
      	if (t === void 0)
      		throw new Et('signal', 'AbortSignal', t);
      	(Pn(t, 'signal'), fl(n, 'listener'));
      	let r;
      	return (
      		t.aborted
      			? queueMicrotask(() => n())
      			: (t.addEventListener('abort', n, { __proto__: null, once: !0, [tl]: !0 }),
      				(r = o(() => {
      					t.removeEventListener('abort', n);
      				}, 'removeEventListener'))),
      		{
      			__proto__: null,
      			[rl]() {
      				let i;
      				(i = r) === null || i === void 0 || i();
      			},
      		}
      	);
      }, 'addAbortListener'),
		AbortSignalAny:
      il.any
      || o((t) => {
      	if (t.length === 1)
      		return t[0];
      	const n = new ol();
      	const r = o(() => n.abort(), 'abort');
      	return (
      		t.forEach((i) => {
      			(Pn(i, 'signals'), i.addEventListener('abort', r, { once: !0 }));
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
	Rt.exports.promisify.custom = Symbol.for('nodejs.util.promisify.custom');
});
const we = E((rd, Un) => {
	'use strict';
	const {
		ArrayIsArray: At,
		ArrayPrototypeIncludes: Cn,
		ArrayPrototypeJoin: vn,
		ArrayPrototypeMap: ul,
		NumberIsInteger: Tt,
		NumberIsNaN: sl,
		NumberMAX_SAFE_INTEGER: dl,
		NumberMIN_SAFE_INTEGER: cl,
		NumberParseInt: hl,
		ObjectPrototypeHasOwnProperty: bl,
		RegExpPrototypeExec: $n,
		String: pl,
		StringPrototypeToUpperCase: _l,
		StringPrototypeTrim: yl,
	} = T();
	const {
		hideStackFrames: C,
		codes: {
			ERR_SOCKET_BAD_PORT: wl,
			ERR_INVALID_ARG_TYPE: P,
			ERR_INVALID_ARG_VALUE: ye,
			ERR_OUT_OF_RANGE: oe,
			ERR_UNKNOWN_SIGNAL: On,
		},
	} = L();
	const { normalizeEncoding: gl } = O();
	const { isAsyncFunction: Sl, isArrayBufferView: El } = O().types;
	const Wn = {};
	function Rl(e) {
		return e === (e | 0);
	}
	o(Rl, 'isInt32');
	function ml(e) {
		return e === e >>> 0;
	}
	o(ml, 'isUint32');
	const Al = /^[0-7]+$/;
	const Tl = 'must be a 32-bit unsigned integer or an octal string';
	function Il(e, t, n) {
		if ((typeof e > 'u' && (e = n), typeof e == 'string')) {
			if ($n(Al, e) === null)
				throw new ye(t, e, Tl);
			e = hl(e, 8);
		}
		return (jn(e, t), e);
	}
	o(Il, 'parseFileMode');
	const Ml = C((e, t, n = cl, r = dl) => {
		if (typeof e != 'number')
			throw new P(t, 'number', e);
		if (!Tt(e))
			throw new oe(t, 'an integer', e);
		if (e < n || e > r)
			throw new oe(t, `>= ${n} && <= ${r}`, e);
	});
	const ql = C((e, t, n = -2147483648, r = 2147483647) => {
		if (typeof e != 'number')
			throw new P(t, 'number', e);
		if (!Tt(e))
			throw new oe(t, 'an integer', e);
		if (e < n || e > r)
			throw new oe(t, `>= ${n} && <= ${r}`, e);
	});
	var jn = C((e, t, n = !1) => {
		if (typeof e != 'number')
			throw new P(t, 'number', e);
		if (!Tt(e))
			throw new oe(t, 'an integer', e);
		const r = n ? 1 : 0;
		const i = 4294967295;
		if (e < r || e > i)
			throw new oe(t, `>= ${r} && <= ${i}`, e);
	});
	function It(e, t) {
		if (typeof e != 'string')
			throw new P(t, 'string', e);
	}
	o(It, 'validateString');
	function kl(e, t, n = void 0, r) {
		if (typeof e != 'number')
			throw new P(t, 'number', e);
		if ((n != null && e < n) || (r != null && e > r) || ((n != null || r != null) && sl(e))) {
			throw new oe(
				t,
				`${n != null ? `>= ${n}` : ''}${n != null && r != null ? ' && ' : ''}${r != null ? `<= ${r}` : ''}`,
				e,
			);
		}
	}
	o(kl, 'validateNumber');
	const Nl = C((e, t, n) => {
		if (!Cn(n, e)) {
			const i
				= `must be one of: ${
					vn(
						ul(n, l => (typeof l == 'string' ? `'${l}'` : pl(l))),
						', ',
					)}`;
			throw new ye(t, e, i);
		}
	});
	function Fn(e, t) {
		if (typeof e != 'boolean')
			throw new P(t, 'boolean', e);
	}
	o(Fn, 'validateBoolean');
	function mt(e, t, n) {
		return e == null || !bl(e, t) ? n : e[t];
	}
	o(mt, 'getOwnPropertyValueOrDefault');
	const Dl = C((e, t, n = null) => {
		const r = mt(n, 'allowArray', !1);
		const i = mt(n, 'allowFunction', !1);
		if (
			(!mt(n, 'nullable', !1) && e === null)
			|| (!r && At(e))
			|| (typeof e != 'object' && (!i || typeof e != 'function'))
		) {
			throw new P(t, 'Object', e);
		}
	});
	const Ll = C((e, t) => {
		if (e != null && typeof e != 'object' && typeof e != 'function')
			throw new P(t, 'a dictionary', e);
	});
	const He = C((e, t, n = 0) => {
		if (!At(e))
			throw new P(t, 'Array', e);
		if (e.length < n) {
			const r = `must be longer than ${n}`;
			throw new ye(t, e, r);
		}
	});
	function Pl(e, t) {
		He(e, t);
		for (let n = 0; n < e.length; n++) It(e[n], `${t}[${n}]`);
	}
	o(Pl, 'validateStringArray');
	function Ol(e, t) {
		He(e, t);
		for (let n = 0; n < e.length; n++) Fn(e[n], `${t}[${n}]`);
	}
	o(Ol, 'validateBooleanArray');
	function Wl(e, t) {
		He(e, t);
		for (let n = 0; n < e.length; n++) {
			const r = e[n];
			const i = `${t}[${n}]`;
			if (r == null)
				throw new P(i, 'AbortSignal', r);
			Bn(r, i);
		}
	}
	o(Wl, 'validateAbortSignalArray');
	function xl(e, t = 'signal') {
		if ((It(e, t), Wn[e] === void 0)) {
			throw Wn[_l(e)] !== void 0
				? new On(`${e} (signals must use all capital letters)`)
				: new On(e);
		}
	}
	o(xl, 'validateSignalName');
	const Cl = C((e, t = 'buffer') => {
		if (!El(e))
			throw new P(t, ['Buffer', 'TypedArray', 'DataView'], e);
	});
	function vl(e, t) {
		const n = gl(t);
		const r = e.length;
		if (n === 'hex' && r % 2 !== 0)
			throw new ye('encoding', t, `is invalid for data of length ${r}`);
	}
	o(vl, 'validateEncoding');
	function $l(e, t = 'Port', n = !0) {
		if (
			(typeof e != 'number' && typeof e != 'string')
			|| (typeof e == 'string' && yl(e).length === 0)
			|| +e !== +e >>> 0
			|| e > 65535
			|| (e === 0 && !n)
		) {
			throw new wl(t, e, n);
		}
		return e | 0;
	}
	o($l, 'validatePort');
	var Bn = C((e, t) => {
		if (e !== void 0 && (e === null || typeof e != 'object' || !('aborted' in e)))
			throw new P(t, 'AbortSignal', e);
	});
	const jl = C((e, t) => {
		if (typeof e != 'function')
			throw new P(t, 'Function', e);
	});
	const Fl = C((e, t) => {
		if (typeof e != 'function' || Sl(e))
			throw new P(t, 'Function', e);
	});
	const Bl = C((e, t) => {
		if (e !== void 0)
			throw new P(t, 'undefined', e);
	});
	function Ul(e, t, n) {
		if (!Cn(n, e))
			throw new P(t, `('${vn(n, '|')}')`, e);
	}
	o(Ul, 'validateUnion');
	const Hl = /^<[^>]*>(?:\s*;\s*[^;"\s]+(?:=(")?[^;"\s]*\1)?)*$/;
	function xn(e, t) {
		if (typeof e > 'u' || !$n(Hl, e)) {
			throw new ye(
				t,
				e,
				'must be an array or string of format "</styles.css>; rel=preload; as=style"',
			);
		}
	}
	o(xn, 'validateLinkHeaderFormat');
	function Gl(e) {
		if (typeof e == 'string')
			return (xn(e, 'hints'), e);
		if (At(e)) {
			const t = e.length;
			let n = '';
			if (t === 0)
				return n;
			for (let r = 0; r < t; r++) {
				const i = e[r];
				(xn(i, 'hints'), (n += i), r !== t - 1 && (n += ', '));
			}
			return n;
		}
		throw new ye(
			'hints',
			e,
			'must be an array or string of format "</styles.css>; rel=preload; as=style"',
		);
	}
	o(Gl, 'validateLinkHeaderValue');
	Un.exports = {
		isInt32: Rl,
		isUint32: ml,
		parseFileMode: Il,
		validateArray: He,
		validateStringArray: Pl,
		validateBooleanArray: Ol,
		validateAbortSignalArray: Wl,
		validateBoolean: Fn,
		validateBuffer: Cl,
		validateDictionary: Ll,
		validateEncoding: vl,
		validateFunction: jl,
		validateInt32: ql,
		validateInteger: Ml,
		validateNumber: kl,
		validateObject: Dl,
		validateOneOf: Nl,
		validatePlainFunction: Fl,
		validatePort: $l,
		validateSignalName: xl,
		validateString: It,
		validateUint32: jn,
		validateUndefined: Bl,
		validateUnion: Ul,
		validateAbortSignal: Bn,
		validateLinkHeaderValue: Gl,
	};
});
const U = E((od, ir) => {
	'use strict';
	const { SymbolAsyncIterator: Hn, SymbolIterator: Gn, SymbolFor: le } = T();
	const Vn = le('nodejs.stream.destroyed');
	const Yn = le('nodejs.stream.errored');
	const Mt = le('nodejs.stream.readable');
	const qt = le('nodejs.stream.writable');
	const Kn = le('nodejs.stream.disturbed');
	const Vl = le('nodejs.webstream.isClosedPromise');
	const Yl = le('nodejs.webstream.controllerErrorFunction');
	function Ge(e, t = !1) {
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
	o(Ge, 'isReadableNodeStream');
	function Ve(e) {
		let t;
		return !!(
			e
			&& typeof e.write == 'function'
			&& typeof e.on == 'function'
			&& (!e._readableState
				|| ((t = e._writableState) === null || t === void 0 ? void 0 : t.writable) !== !1)
		);
	}
	o(Ve, 'isWritableNodeStream');
	function Kl(e) {
		return !!(
			e
			&& typeof e.pipe == 'function'
			&& e._readableState
			&& typeof e.on == 'function'
			&& typeof e.write == 'function'
		);
	}
	o(Kl, 'isDuplexNodeStream');
	function B(e) {
		return (
			e
			&& (e._readableState
				|| e._writableState
				|| (typeof e.write == 'function' && typeof e.on == 'function')
				|| (typeof e.pipe == 'function' && typeof e.on == 'function'))
		);
	}
	o(B, 'isNodeStream');
	function zn(e) {
		return !!(
			e
			&& !B(e)
			&& typeof e.pipeThrough == 'function'
			&& typeof e.getReader == 'function'
			&& typeof e.cancel == 'function'
		);
	}
	o(zn, 'isReadableStream');
	function Xn(e) {
		return !!(e && !B(e) && typeof e.getWriter == 'function' && typeof e.abort == 'function');
	}
	o(Xn, 'isWritableStream');
	function Jn(e) {
		return !!(e && !B(e) && typeof e.readable == 'object' && typeof e.writable == 'object');
	}
	o(Jn, 'isTransformStream');
	function zl(e) {
		return zn(e) || Xn(e) || Jn(e);
	}
	o(zl, 'isWebStream');
	function Xl(e, t) {
		return e == null
			? !1
			: t === !0
				? typeof e[Hn] == 'function'
				: t === !1
					? typeof e[Gn] == 'function'
					: typeof e[Hn] == 'function' || typeof e[Gn] == 'function';
	}
	o(Xl, 'isIterable');
	function Ye(e) {
		if (!B(e))
			return null;
		const t = e._writableState;
		const n = e._readableState;
		const r = t || n;
		return !!(e.destroyed || e[Vn] || (r != null && r.destroyed));
	}
	o(Ye, 'isDestroyed');
	function Qn(e) {
		if (!Ve(e))
			return null;
		if (e.writableEnded === !0)
			return !0;
		const t = e._writableState;
		return t != null && t.errored ? !1 : typeof t?.ended != 'boolean' ? null : t.ended;
	}
	o(Qn, 'isWritableEnded');
	function Jl(e, t) {
		if (!Ve(e))
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
	o(Jl, 'isWritableFinished');
	function Ql(e) {
		if (!Ge(e))
			return null;
		if (e.readableEnded === !0)
			return !0;
		const t = e._readableState;
		return !t || t.errored ? !1 : typeof t?.ended != 'boolean' ? null : t.ended;
	}
	o(Ql, 'isReadableEnded');
	function Zn(e, t) {
		if (!Ge(e))
			return null;
		const n = e._readableState;
		return n != null && n.errored
			? !1
			: typeof n?.endEmitted != 'boolean'
				? null
				: !!(n.endEmitted || (t === !1 && n.ended === !0 && n.length === 0));
	}
	o(Zn, 'isReadableFinished');
	function er(e) {
		return e && e[Mt] != null
			? e[Mt]
			: typeof e?.readable != 'boolean'
				? null
				: Ye(e)
					? !1
					: Ge(e) && e.readable && !Zn(e);
	}
	o(er, 'isReadable');
	function tr(e) {
		return e && e[qt] != null
			? e[qt]
			: typeof e?.writable != 'boolean'
				? null
				: Ye(e)
					? !1
					: Ve(e) && e.writable && !Qn(e);
	}
	o(tr, 'isWritable');
	function Zl(e, t) {
		return B(e)
			? Ye(e)
				? !0
				: !((t?.readable !== !1 && er(e)) || (t?.writable !== !1 && tr(e)))
			: null;
	}
	o(Zl, 'isFinished');
	function ea(e) {
		let t, n;
		return B(e)
			? e.writableErrored
				? e.writableErrored
				: (t = (n = e._writableState) === null || n === void 0 ? void 0 : n.errored) !== null
					&& t !== void 0
						? t
						: null
			: null;
	}
	o(ea, 'isWritableErrored');
	function ta(e) {
		let t, n;
		return B(e)
			? e.readableErrored
				? e.readableErrored
				: (t = (n = e._readableState) === null || n === void 0 ? void 0 : n.errored) !== null
					&& t !== void 0
						? t
						: null
			: null;
	}
	o(ta, 'isReadableErrored');
	function na(e) {
		if (!B(e))
			return null;
		if (typeof e.closed == 'boolean')
			return e.closed;
		const t = e._writableState;
		const n = e._readableState;
		return typeof t?.closed == 'boolean' || typeof n?.closed == 'boolean'
			? t?.closed || n?.closed
			: typeof e._closed == 'boolean' && nr(e)
				? e._closed
				: null;
	}
	o(na, 'isClosed');
	function nr(e) {
		return (
			typeof e._closed == 'boolean'
			&& typeof e._defaultKeepAlive == 'boolean'
			&& typeof e._removedConnection == 'boolean'
			&& typeof e._removedContLen == 'boolean'
		);
	}
	o(nr, 'isOutgoingMessage');
	function rr(e) {
		return typeof e._sent100 == 'boolean' && nr(e);
	}
	o(rr, 'isServerResponse');
	function ra(e) {
		let t;
		return (
			typeof e._consuming == 'boolean'
			&& typeof e._dumped == 'boolean'
			&& ((t = e.req) === null || t === void 0 ? void 0 : t.upgradeOrConnect) === void 0
		);
	}
	o(ra, 'isServerRequest');
	function ia(e) {
		if (!B(e))
			return null;
		const t = e._writableState;
		const n = e._readableState;
		const r = t || n;
		return (!r && rr(e)) || !!(r && r.autoDestroy && r.emitClose && r.closed === !1);
	}
	o(ia, 'willEmitClose');
	function oa(e) {
		let t;
		return !!(
			e && ((t = e[Kn]) !== null && t !== void 0 ? t : e.readableDidRead || e.readableAborted)
		);
	}
	o(oa, 'isDisturbed');
	function la(e) {
		let t, n, r, i, l, a, u, s, f, d;
		return !!(
			e
			&& ((t
				= (n
					= (r
						= (i
							= (l = (a = e[Yn]) !== null && a !== void 0 ? a : e.readableErrored) !== null
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
	o(la, 'isErrored');
	ir.exports = {
		isDestroyed: Ye,
		kIsDestroyed: Vn,
		isDisturbed: oa,
		kIsDisturbed: Kn,
		isErrored: la,
		kIsErrored: Yn,
		isReadable: er,
		kIsReadable: Mt,
		kIsClosedPromise: Vl,
		kControllerErrorFunction: Yl,
		kIsWritable: qt,
		isClosed: na,
		isDuplexNodeStream: Kl,
		isFinished: Zl,
		isIterable: Xl,
		isReadableNodeStream: Ge,
		isReadableStream: zn,
		isReadableEnded: Ql,
		isReadableFinished: Zn,
		isReadableErrored: ta,
		isNodeStream: B,
		isWebStream: zl,
		isWritable: tr,
		isWritableNodeStream: Ve,
		isWritableStream: Xn,
		isWritableEnded: Qn,
		isWritableFinished: Jl,
		isWritableErrored: ea,
		isServerRequest: ra,
		isServerResponse: rr,
		willEmitClose: ia,
		isTransformStream: Jn,
	};
});
const H = E((ad, Pt) => {
	'use strict';
	const Z = k('process');
	const { AbortError: hr, codes: aa } = L();
	const { ERR_INVALID_ARG_TYPE: fa, ERR_STREAM_PREMATURE_CLOSE: or } = aa;
	const { kEmptyObject: Nt, once: Dt } = O();
	const {
		validateAbortSignal: ua,
		validateFunction: sa,
		validateObject: da,
		validateBoolean: ca,
	} = we();
	const { Promise: ha, PromisePrototypeThen: ba, SymbolDispose: br } = T();
	const {
		isClosed: pa,
		isReadable: lr,
		isReadableNodeStream: kt,
		isReadableStream: _a,
		isReadableFinished: ar,
		isReadableErrored: fr,
		isWritable: ur,
		isWritableNodeStream: sr,
		isWritableStream: ya,
		isWritableFinished: dr,
		isWritableErrored: cr,
		isNodeStream: wa,
		willEmitClose: ga,
		kIsClosedPromise: Sa,
	} = U();
	let ge;
	function Ea(e) {
		return e.setHeader && typeof e.abort == 'function';
	}
	o(Ea, 'isRequest');
	const Lt = o(() => {}, 'nop');
	function pr(e, t, n) {
		let r, i;
		if (
			(arguments.length === 2 ? ((n = t), (t = Nt)) : t == null ? (t = Nt) : da(t, 'options'),
			sa(n, 'callback'),
			ua(t.signal, 'options.signal'),
			(n = Dt(n)),
			_a(e) || ya(e))) {
			return Ra(e, t, n);
		}
		if (!wa(e))
			throw new fa('stream', ['ReadableStream', 'WritableStream', 'Stream'], e);
		const l = (r = t.readable) !== null && r !== void 0 ? r : kt(e);
		const a = (i = t.writable) !== null && i !== void 0 ? i : sr(e);
		const u = e._writableState;
		const s = e._readableState;
		const f = o(() => {
			e.writable || p();
		}, 'onlegacyfinish');
		let d = ga(e) && kt(e) === l && sr(e) === a;
		let c = dr(e, !1);
		let p = o(() => {
			((c = !0), e.destroyed && (d = !1), !(d && (!e.readable || l)) && (!l || h) && n.call(e));
		}, 'onfinish');
		let h = ar(e, !1);
		const S = o(() => {
			((h = !0), e.destroyed && (d = !1), !(d && (!e.writable || a)) && (!a || c) && n.call(e));
		}, 'onend');
		const b = o((M) => {
			n.call(e, M);
		}, 'onerror');
		let R = pa(e);
		const y = o(() => {
			R = !0;
			const M = cr(e) || fr(e);
			if (M && typeof M != 'boolean')
				return n.call(e, M);
			if (l && !h && kt(e, !0) && !ar(e, !1))
				return n.call(e, new or());
			if (a && !c && !dr(e, !1))
				return n.call(e, new or());
			n.call(e);
		}, 'onclose');
		const m = o(() => {
			R = !0;
			const M = cr(e) || fr(e);
			if (M && typeof M != 'boolean')
				return n.call(e, M);
			n.call(e);
		}, 'onclosed');
		const q = o(() => {
			e.req.on('finish', p);
		}, 'onrequest');
		(Ea(e)
			? (e.on('complete', p), d || e.on('abort', y), e.req ? q() : e.on('request', q))
			: a && !u && (e.on('end', f), e.on('close', f)),
		!d && typeof e.aborted == 'boolean' && e.on('aborted', y),
		e.on('end', S),
		e.on('finish', p),
		t.error !== !1 && e.on('error', b),
		e.on('close', y),
		R
			? Z.nextTick(y)
			: (u != null && u.errorEmitted) || (s != null && s.errorEmitted)
					? d || Z.nextTick(m)
					: ((!l && (!d || lr(e)) && (c || ur(e) === !1))
						|| (!a && (!d || ur(e)) && (h || lr(e) === !1))
						|| (s && e.req && e.aborted))
					&& Z.nextTick(m));
		const g = o(() => {
			((n = Lt),
			e.removeListener('aborted', y),
			e.removeListener('complete', p),
			e.removeListener('abort', y),
			e.removeListener('request', q),
			e.req && e.req.removeListener('finish', p),
			e.removeListener('end', f),
			e.removeListener('close', f),
			e.removeListener('finish', p),
			e.removeListener('end', S),
			e.removeListener('error', b),
			e.removeListener('close', y));
		}, 'cleanup');
		if (t.signal && !R) {
			const M = o(() => {
				const ne = n;
				(g(), ne.call(e, new hr(void 0, { cause: t.signal.reason })));
			}, 'abort');
			if (t.signal.aborted) {
				Z.nextTick(M);
			}
			else {
				ge = ge || O().addAbortListener;
				const ne = ge(t.signal, M);
				const x = n;
				n = Dt((...pe) => {
					(ne[br](), x.apply(e, pe));
				});
			}
		}
		return g;
	}
	o(pr, 'eos');
	function Ra(e, t, n) {
		let r = !1;
		let i = Lt;
		if (t.signal) {
			if (
				((i = o(() => {
					((r = !0), n.call(e, new hr(void 0, { cause: t.signal.reason })));
				}, 'abort')),
				t.signal.aborted)) {
				Z.nextTick(i);
			}
			else {
				ge = ge || O().addAbortListener;
				const a = ge(t.signal, i);
				const u = n;
				n = Dt((...s) => {
					(a[br](), u.apply(e, s));
				});
			}
		}
		const l = o((...a) => {
			r || Z.nextTick(() => n.apply(e, a));
		}, 'resolverFn');
		return (ba(e[Sa].promise, l, l), Lt);
	}
	o(Ra, 'eosWeb');
	function ma(e, t) {
		let n;
		let r = !1;
		return (
			t === null && (t = Nt),
			(n = t) !== null && n !== void 0 && n.cleanup && (ca(t.cleanup, 'cleanup'), (r = t.cleanup)),
			new ha((i, l) => {
				const a = pr(e, t, (u) => {
					(r && a(), u ? l(u) : i());
				});
			})
		);
	}
	o(ma, 'finished');
	Pt.exports = pr;
	Pt.exports.finished = ma;
});
const ae = E((ud, mr) => {
	'use strict';
	const G = k('process');
	const {
		aggregateTwoErrors: Aa,
		codes: { ERR_MULTIPLE_CALLBACK: Ta },
		AbortError: Ia,
	} = L();
	const { Symbol: wr } = T();
	const { kIsDestroyed: Ma, isDestroyed: qa, isFinished: ka, isServerRequest: Na } = U();
	const gr = wr('kDestroy');
	const Ot = wr('kConstruct');
	function Sr(e, t, n) {
		e && (e.stack, t && !t.errored && (t.errored = e), n && !n.errored && (n.errored = e));
	}
	o(Sr, 'checkError');
	function Da(e, t) {
		const n = this._readableState;
		const r = this._writableState;
		const i = r || n;
		return (r != null && r.destroyed) || (n != null && n.destroyed)
			? (typeof t == 'function' && t(), this)
			: (Sr(e, r, n),
				r && (r.destroyed = !0),
				n && (n.destroyed = !0),
				i.constructed
					? _r(this, e, t)
					: this.once(gr, function (l) {
							_r(this, Aa(l, e), t);
						}),
				this);
	}
	o(Da, 'destroy');
	function _r(e, t, n) {
		let r = !1;
		function i(l) {
			if (r)
				return;
			r = !0;
			const a = e._readableState;
			const u = e._writableState;
			(Sr(l, u, a),
			u && (u.closed = !0),
			a && (a.closed = !0),
			typeof n == 'function' && n(l),
			l ? G.nextTick(La, e, l) : G.nextTick(Er, e));
		}
		o(i, 'onDestroy');
		try {
			e._destroy(t || null, i);
		}
		catch (l) {
			i(l);
		}
	}
	o(_r, '_destroy');
	function La(e, t) {
		(Wt(e, t), Er(e));
	}
	o(La, 'emitErrorCloseNT');
	function Er(e) {
		const t = e._readableState;
		const n = e._writableState;
		(n && (n.closeEmitted = !0),
		t && (t.closeEmitted = !0),
		((n != null && n.emitClose) || (t != null && t.emitClose)) && e.emit('close'));
	}
	o(Er, 'emitCloseNT');
	function Wt(e, t) {
		const n = e._readableState;
		const r = e._writableState;
		(r != null && r.errorEmitted)
		|| (n != null && n.errorEmitted)
		|| (r && (r.errorEmitted = !0), n && (n.errorEmitted = !0), e.emit('error', t));
	}
	o(Wt, 'emitErrorNT');
	function Pa() {
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
	o(Pa, 'undestroy');
	function xt(e, t, n) {
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
				n ? G.nextTick(Wt, e, t) : Wt(e, t));
	}
	o(xt, 'errorOrDestroy');
	function Oa(e, t) {
		if (typeof e._construct != 'function')
			return;
		const n = e._readableState;
		const r = e._writableState;
		(n && (n.constructed = !1),
		r && (r.constructed = !1),
		e.once(Ot, t),
		!(e.listenerCount(Ot) > 1) && G.nextTick(Wa, e));
	}
	o(Oa, 'construct');
	function Wa(e) {
		let t = !1;
		function n(r) {
			if (t) {
				xt(e, r ?? new Ta());
				return;
			}
			t = !0;
			const i = e._readableState;
			const l = e._writableState;
			const a = l || i;
			(i && (i.constructed = !0),
			l && (l.constructed = !0),
			a.destroyed ? e.emit(gr, r) : r ? xt(e, r, !0) : G.nextTick(xa, e));
		}
		o(n, 'onConstruct');
		try {
			e._construct((r) => {
				G.nextTick(n, r);
			});
		}
		catch (r) {
			G.nextTick(n, r);
		}
	}
	o(Wa, 'constructNT');
	function xa(e) {
		e.emit(Ot);
	}
	o(xa, 'emitConstructNT');
	function yr(e) {
		return e?.setHeader && typeof e.abort == 'function';
	}
	o(yr, 'isRequest');
	function Rr(e) {
		e.emit('close');
	}
	o(Rr, 'emitCloseLegacy');
	function Ca(e, t) {
		(e.emit('error', t), G.nextTick(Rr, e));
	}
	o(Ca, 'emitErrorCloseLegacy');
	function va(e, t) {
		!e
		|| qa(e)
		|| (!t && !ka(e) && (t = new Ia()),
		Na(e)
			? ((e.socket = null), e.destroy(t))
			: yr(e)
				? e.abort()
				: yr(e.req)
					? e.req.abort()
					: typeof e.destroy == 'function'
						? e.destroy(t)
						: typeof e.close == 'function'
							? e.close()
							: t
								? G.nextTick(Ca, e, t)
								: G.nextTick(Rr, e),
		e.destroyed || (e[Ma] = !0));
	}
	o(va, 'destroyer');
	mr.exports = { construct: Oa, destroyer: va, destroy: Da, undestroy: Pa, errorOrDestroy: xt };
});
const Xe = E((dd, Tr) => {
	'use strict';
	const { ArrayIsArray: $a, ObjectSetPrototypeOf: Ar } = T();
	const { EventEmitter: Ke } = k('events');
	function ze(e) {
		Ke.call(this, e);
	}
	o(ze, 'Stream');
	Ar(ze.prototype, Ke.prototype);
	Ar(ze, Ke);
	ze.prototype.pipe = function (e, t) {
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
			(f(), Ke.listenerCount(this, 'error') === 0 && this.emit('error', d));
		}
		(o(s, 'onerror'), Ct(n, 'error', s), Ct(e, 'error', s));
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
	function Ct(e, t, n) {
		if (typeof e.prependListener == 'function')
			return e.prependListener(t, n);
		!e._events || !e._events[t]
			? e.on(t, n)
			: $a(e._events[t])
				? e._events[t].unshift(n)
				: (e._events[t] = [n, e._events[t]]);
	}
	o(Ct, 'prependListener');
	Tr.exports = { Stream: ze, prependListener: Ct };
});
const ke = E((hd, Je) => {
	'use strict';
	const { SymbolDispose: ja } = T();
	const { AbortError: Ir, codes: Fa } = L();
	const { isNodeStream: Mr, isWebStream: Ba, kControllerErrorFunction: Ua } = U();
	const Ha = H();
	const { ERR_INVALID_ARG_TYPE: qr } = Fa;
	let vt;
	const Ga = o((e, t) => {
		if (typeof e != 'object' || !('aborted' in e))
			throw new qr(t, 'AbortSignal', e);
	}, 'validateAbortSignal');
	Je.exports.addAbortSignal = o((t, n) => {
		if ((Ga(t, 'signal'), !Mr(n) && !Ba(n)))
			throw new qr('stream', ['ReadableStream', 'WritableStream', 'Stream'], n);
		return Je.exports.addAbortSignalNoValidate(t, n);
	}, 'addAbortSignal');
	Je.exports.addAbortSignalNoValidate = function (e, t) {
		if (typeof e != 'object' || !('aborted' in e))
			return t;
		const n = Mr(t)
			? () => {
					t.destroy(new Ir(void 0, { cause: e.reason }));
				}
			: () => {
					t[Ua](new Ir(void 0, { cause: e.reason }));
				};
		if (e.aborted) {
			n();
		}
		else {
			vt = vt || O().addAbortListener;
			const r = vt(e, n);
			Ha(t, r[ja]);
		}
		return t;
	};
});
const Dr = E((_d, Nr) => {
	'use strict';
	const {
		StringPrototypeSlice: kr,
		SymbolIterator: Va,
		TypedArrayPrototypeSet: Qe,
		Uint8Array: Ya,
	} = T();
	const { Buffer: $t } = k('buffer');
	const { inspect: Ka } = O();
	Nr.exports = class {
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
				return $t.alloc(0);
			const n = $t.allocUnsafe(t >>> 0);
			let r = this.head;
			let i = 0;
			for (; r;) (Qe(n, r.data, i), (i += r.data.length), (r = r.next));
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

		* [Va]() {
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
						: ((n += kr(l, 0, t)), (this.head = r), (r.data = kr(l, t)));
					break;
				}
				++i;
			} while ((r = r.next) !== null);
			return ((this.length -= i), n);
		}

		_getBuffer(t) {
			const n = $t.allocUnsafe(t);
			const r = t;
			let i = this.head;
			let l = 0;
			do {
				const a = i.data;
				if (t > a.length) {
					(Qe(n, a, r - t), (t -= a.length));
				}
				else {
					t === a.length
						? (Qe(n, a, r - t), ++l, i.next ? (this.head = i.next) : (this.head = this.tail = null))
						: (Qe(n, new Ya(a.buffer, a.byteOffset, t), r - t),
							(this.head = i),
							(i.data = a.slice(t)));
					break;
				}
				++l;
			} while ((i = i.next) !== null);
			return ((this.length -= l), n);
		}

		[Symbol.for('nodejs.util.inspect.custom')](t, n) {
			return Ka(this, { ...n, depth: 0, customInspect: !1 });
		}
	};
});
const Ne = E((wd, Wr) => {
	'use strict';
	const { MathFloor: za, NumberIsInteger: Xa } = T();
	const { validateInteger: Ja } = we();
	const { ERR_INVALID_ARG_VALUE: Qa } = L().codes;
	let Lr = 16 * 1024;
	let Pr = 16;
	function Za(e, t, n) {
		return e.highWaterMark ?? t ? e[n] : null;
	}
	o(Za, 'highWaterMarkFrom');
	function Or(e) {
		return e ? Pr : Lr;
	}
	o(Or, 'getDefaultHighWaterMark');
	function ef(e, t) {
		(Ja(t, 'value', 0), e ? (Pr = t) : (Lr = t));
	}
	o(ef, 'setDefaultHighWaterMark');
	function tf(e, t, n, r) {
		const i = Za(t, r, n);
		if (i != null) {
			if (!Xa(i) || i < 0) {
				const l = r ? `options.${n}` : 'options.highWaterMark';
				throw new Qa(l, i);
			}
			return za(i);
		}
		return Or(e.objectMode);
	}
	o(tf, 'getHighWaterMark');
	Wr.exports = { getHighWaterMark: tf, getDefaultHighWaterMark: Or, setDefaultHighWaterMark: ef };
});
const jt = E((Sd, $r) => {
	'use strict';
	const xr = k('process');
	const { PromisePrototypeThen: nf, SymbolAsyncIterator: Cr, SymbolIterator: vr } = T();
	const { Buffer: rf } = k('buffer');
	const { ERR_INVALID_ARG_TYPE: of, ERR_STREAM_NULL_VALUES: lf } = L().codes;
	function af(e, t, n) {
		let r;
		if (typeof t == 'string' || t instanceof rf) {
			return new e({
				objectMode: !0,
				...n,
				read() {
					(this.push(t), this.push(null));
				},
			});
		}
		let i;
		if (t && t[Cr])
			((i = !0), (r = t[Cr]()));
		else if (t && t[vr])
			((i = !1), (r = t[vr]()));
		else throw new of('iterable', ['Iterable'], t);
		const l = new e({ objectMode: !0, highWaterMark: 1, ...n });
		let a = !1;
		((l._read = function () {
			a || ((a = !0), s());
		}),
		(l._destroy = function (f, d) {
			nf(
				u(f),
				() => xr.nextTick(d, f),
				c => xr.nextTick(d, c || f),
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
							throw ((a = !1), new lf());
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
	o(af, 'from');
	$r.exports = af;
});
const Le = E((Rd, ri) => {
	'use strict';
	const j = k('process');
	const {
		ArrayPrototypeIndexOf: ff,
		NumberIsInteger: uf,
		NumberIsNaN: sf,
		NumberParseInt: df,
		ObjectDefineProperties: Kt,
		ObjectKeys: cf,
		ObjectSetPrototypeOf: Br,
		Promise: Ur,
		SafeSet: hf,
		SymbolAsyncDispose: bf,
		SymbolAsyncIterator: pf,
		Symbol: _f,
	} = T();
	ri.exports = _;
	_.ReadableState = nt;
	const { EventEmitter: yf } = k('events');
	const { Stream: ee, prependListener: wf } = Xe();
	const { Buffer: Ft } = k('buffer');
	const { addAbortSignal: gf } = ke();
	const Hr = H();
	var w = O().debuglog('stream', (e) => {
		w = e;
	});
	const Sf = Dr();
	const Re = ae();
	const { getHighWaterMark: Ef, getDefaultHighWaterMark: Rf } = Ne();
	const {
		aggregateTwoErrors: jr,
		codes: {
			ERR_INVALID_ARG_TYPE: mf,
			ERR_METHOD_NOT_IMPLEMENTED: Af,
			ERR_OUT_OF_RANGE: Tf,
			ERR_STREAM_PUSH_AFTER_EOF: If,
			ERR_STREAM_UNSHIFT_AFTER_END_EVENT: Mf,
		},
		AbortError: qf,
	} = L();
	const { validateObject: kf } = we();
	const fe = _f('kPaused');
	const { StringDecoder: Gr } = k('string_decoder/');
	const Nf = jt();
	Br(_.prototype, ee.prototype);
	Br(_, ee);
	const Bt = o(() => {}, 'nop');
	const { errorOrDestroy: Se } = Re;
	const Ee = 1;
	const Df = 2;
	const Vr = 4;
	const De = 8;
	const Yr = 16;
	const Ze = 32;
	const et = 64;
	const Kr = 128;
	const Lf = 256;
	const Pf = 512;
	const Of = 1024;
	const Vt = 2048;
	const Yt = 4096;
	const Wf = 8192;
	const xf = 16384;
	const Cf = 32768;
	const zr = 65536;
	const vf = 1 << 17;
	const $f = 1 << 18;
	function N(e) {
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
	o(N, 'makeBitMapDescriptor');
	Kt(nt.prototype, {
		objectMode: N(Ee),
		ended: N(Df),
		endEmitted: N(Vr),
		reading: N(De),
		constructed: N(Yr),
		sync: N(Ze),
		needReadable: N(et),
		emittedReadable: N(Kr),
		readableListening: N(Lf),
		resumeScheduled: N(Pf),
		errorEmitted: N(Of),
		emitClose: N(Vt),
		autoDestroy: N(Yt),
		destroyed: N(Wf),
		closed: N(xf),
		closeEmitted: N(Cf),
		multiAwaitDrain: N(zr),
		readingMore: N(vf),
		dataEmitted: N($f),
	});
	function nt(e, t, n) {
		(typeof n != 'boolean' && (n = t instanceof V()),
		(this.state = Vt | Yt | Yr | Ze),
		e && e.objectMode && (this.state |= Ee),
		n && e && e.readableObjectMode && (this.state |= Ee),
		(this.highWaterMark = e ? Ef(this, e, 'readableHighWaterMark', n) : Rf(!1)),
		(this.buffer = new Sf()),
		(this.length = 0),
		(this.pipes = []),
		(this.flowing = null),
		(this[fe] = null),
		e && e.emitClose === !1 && (this.state &= ~Vt),
		e && e.autoDestroy === !1 && (this.state &= ~Yt),
		(this.errored = null),
		(this.defaultEncoding = (e && e.defaultEncoding) || 'utf8'),
		(this.awaitDrainWriters = null),
		(this.decoder = null),
		(this.encoding = null),
		e && e.encoding && ((this.decoder = new Gr(e.encoding)), (this.encoding = e.encoding)));
	}
	o(nt, 'ReadableState');
	function _(e) {
		if (!(this instanceof _))
			return new _(e);
		const t = this instanceof V();
		((this._readableState = new nt(e, this, t)),
		e
		&& (typeof e.read == 'function' && (this._read = e.read),
		typeof e.destroy == 'function' && (this._destroy = e.destroy),
		typeof e.construct == 'function' && (this._construct = e.construct),
		e.signal && !t && gf(e.signal, this)),
		ee.call(this, e),
		Re.construct(this, () => {
			this._readableState.needReadable && tt(this, this._readableState);
		}));
	}
	o(_, 'Readable');
	_.prototype.destroy = Re.destroy;
	_.prototype._undestroy = Re.undestroy;
	_.prototype._destroy = function (e, t) {
		t(e);
	};
	_.prototype[yf.captureRejectionSymbol] = function (e) {
		this.destroy(e);
	};
	_.prototype[bf] = function () {
		let e;
		return (
			this.destroyed || ((e = this.readableEnded ? null : new qf()), this.destroy(e)),
			new Ur((t, n) => Hr(this, r => (r && r !== e ? n(r) : t(null))))
		);
	};
	_.prototype.push = function (e, t) {
		return Xr(this, e, t, !1);
	};
	_.prototype.unshift = function (e, t) {
		return Xr(this, e, t, !0);
	};
	function Xr(e, t, n, r) {
		w('readableAddChunk', t);
		const i = e._readableState;
		let l;
		if (
			((i.state & Ee) === 0
				&& (typeof t == 'string'
					? ((n = n || i.defaultEncoding),
						i.encoding !== n
						&& (r && i.encoding
							? (t = Ft.from(t, n).toString(i.encoding))
							: ((t = Ft.from(t, n)), (n = ''))))
					: t instanceof Ft
						? (n = '')
						: ee._isUint8Array(t)
							? ((t = ee._uint8ArrayToBuffer(t)), (n = ''))
							: t != null && (l = new mf('chunk', ['string', 'Buffer', 'Uint8Array'], t))),
			l)) {
			Se(e, l);
		}
		else if (t === null) {
			((i.state &= ~De), Bf(e, i));
		}
		else if ((i.state & Ee) !== 0 || (t && t.length > 0)) {
			if (r) {
				if ((i.state & Vr) !== 0) {
					Se(e, new Mf());
				}
				else {
					if (i.destroyed || i.errored)
						return !1;
					Ut(e, i, t, !0);
				}
			}
			else if (i.ended) {
				Se(e, new If());
			}
			else {
				if (i.destroyed || i.errored)
					return !1;
				((i.state &= ~De),
				i.decoder && !n
					? ((t = i.decoder.write(t)),
						i.objectMode || t.length !== 0 ? Ut(e, i, t, !1) : tt(e, i))
					: Ut(e, i, t, !1));
			}
		}
		else {
			r || ((i.state &= ~De), tt(e, i));
		}
		return !i.ended && (i.length < i.highWaterMark || i.length === 0);
	}
	o(Xr, 'readableAddChunk');
	function Ut(e, t, n, r) {
		(t.flowing && t.length === 0 && !t.sync && e.listenerCount('data') > 0
			? ((t.state & zr) !== 0 ? t.awaitDrainWriters.clear() : (t.awaitDrainWriters = null),
				(t.dataEmitted = !0),
				e.emit('data', n))
			: ((t.length += t.objectMode ? 1 : n.length),
				r ? t.buffer.unshift(n) : t.buffer.push(n),
				(t.state & et) !== 0 && rt(e)),
		tt(e, t));
	}
	o(Ut, 'addChunk');
	_.prototype.isPaused = function () {
		const e = this._readableState;
		return e[fe] === !0 || e.flowing === !1;
	};
	_.prototype.setEncoding = function (e) {
		const t = new Gr(e);
		((this._readableState.decoder = t),
		(this._readableState.encoding = this._readableState.decoder.encoding));
		const n = this._readableState.buffer;
		let r = '';
		for (const i of n) r += t.write(i);
		return (n.clear(), r !== '' && n.push(r), (this._readableState.length = r.length), this);
	};
	const jf = 1073741824;
	function Ff(e) {
		if (e > jf)
			throw new Tf('size', '<= 1GiB', e);
		return (
			e--, (e |= e >>> 1), (e |= e >>> 2), (e |= e >>> 4), (e |= e >>> 8), (e |= e >>> 16), e++, e
		);
	}
	o(Ff, 'computeNewHighWaterMark');
	function Fr(e, t) {
		return e <= 0 || (t.length === 0 && t.ended)
			? 0
			: (t.state & Ee) !== 0
					? 1
					: sf(e)
						? t.flowing && t.length
							? t.buffer.first().length
							: t.length
						: e <= t.length
							? e
							: t.ended
								? t.length
								: 0;
	}
	o(Fr, 'howMuchToRead');
	_.prototype.read = function (e) {
		(w('read', e), e === void 0 ? (e = Number.NaN) : uf(e) || (e = df(e, 10)));
		const t = this._readableState;
		const n = e;
		if (
			(e > t.highWaterMark && (t.highWaterMark = Ff(e)),
			e !== 0 && (t.state &= ~Kr),
			e === 0
			&& t.needReadable
			&& ((t.highWaterMark !== 0 ? t.length >= t.highWaterMark : t.length > 0) || t.ended))) {
			return (
				w('read: emitReadable', t.length, t.ended),
				t.length === 0 && t.ended ? Ht(this) : rt(this),
				null
			);
		}
		if (((e = Fr(e, t)), e === 0 && t.ended))
			return (t.length === 0 && Ht(this), null);
		let r = (t.state & et) !== 0;
		if (
			(w('need readable', r),
			(t.length === 0 || t.length - e < t.highWaterMark)
			&& ((r = !0), w('length less than watermark', r)),
			t.ended || t.reading || t.destroyed || t.errored || !t.constructed)) {
			((r = !1), w('reading, ended or constructing', r));
		}
		else if (r) {
			(w('do read'), (t.state |= De | Ze), t.length === 0 && (t.state |= et));
			try {
				this._read(t.highWaterMark);
			}
			catch (l) {
				Se(this, l);
			}
			((t.state &= ~Ze), t.reading || (e = Fr(n, t)));
		}
		let i;
		return (
			e > 0 ? (i = ti(e, t)) : (i = null),
			i === null
				? ((t.needReadable = t.length <= t.highWaterMark), (e = 0))
				: ((t.length -= e),
					t.multiAwaitDrain ? t.awaitDrainWriters.clear() : (t.awaitDrainWriters = null)),
			t.length === 0 && (t.ended || (t.needReadable = !0), n !== e && t.ended && Ht(this)),
			i !== null
			&& !t.errorEmitted
			&& !t.closeEmitted
			&& ((t.dataEmitted = !0), this.emit('data', i)),
			i
		);
	};
	function Bf(e, t) {
		if ((w('onEofChunk'), !t.ended)) {
			if (t.decoder) {
				const n = t.decoder.end();
				n && n.length && (t.buffer.push(n), (t.length += t.objectMode ? 1 : n.length));
			}
			((t.ended = !0), t.sync ? rt(e) : ((t.needReadable = !1), (t.emittedReadable = !0), Jr(e)));
		}
	}
	o(Bf, 'onEofChunk');
	function rt(e) {
		const t = e._readableState;
		(w('emitReadable', t.needReadable, t.emittedReadable),
		(t.needReadable = !1),
		t.emittedReadable
		|| (w('emitReadable', t.flowing), (t.emittedReadable = !0), j.nextTick(Jr, e)));
	}
	o(rt, 'emitReadable');
	function Jr(e) {
		const t = e._readableState;
		(w('emitReadable_', t.destroyed, t.length, t.ended),
		!t.destroyed
		&& !t.errored
		&& (t.length || t.ended)
		&& (e.emit('readable'), (t.emittedReadable = !1)),
		(t.needReadable = !t.flowing && !t.ended && t.length <= t.highWaterMark),
		Zr(e));
	}
	o(Jr, 'emitReadable_');
	function tt(e, t) {
		!t.readingMore && t.constructed && ((t.readingMore = !0), j.nextTick(Uf, e, t));
	}
	o(tt, 'maybeReadMore');
	function Uf(e, t) {
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
	o(Uf, 'maybeReadMore_');
	_.prototype._read = function (e) {
		throw new Af('_read()');
	};
	_.prototype.pipe = function (e, t) {
		const n = this;
		const r = this._readableState;
		(r.pipes.length === 1
			&& (r.multiAwaitDrain
				|| ((r.multiAwaitDrain = !0),
				(r.awaitDrainWriters = new hf(r.awaitDrainWriters ? [r.awaitDrainWriters] : [])))),
		r.pipes.push(e),
		w('pipe count=%d opts=%j', r.pipes.length, t));
		const l = (!t || t.end !== !1) && e !== j.stdout && e !== j.stderr ? u : R;
		(r.endEmitted ? j.nextTick(l) : n.once('end', l), e.on('unpipe', a));
		function a(y, m) {
			(w('onunpipe'), y === n && m && m.hasUnpiped === !1 && ((m.hasUnpiped = !0), d()));
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
			n.removeListener('end', R),
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
			s || ((s = Hf(n, e)), e.on('drain', s)));
		}
		(o(c, 'pause'), n.on('data', p));
		function p(y) {
			w('ondata');
			const m = e.write(y);
			(w('dest.write', m), m === !1 && c());
		}
		o(p, 'ondata');
		function h(y) {
			if ((w('onerror', y), R(), e.removeListener('error', h), e.listenerCount('error') === 0)) {
				const m = e._writableState || e._readableState;
				m && !m.errorEmitted ? Se(e, y) : e.emit('error', y);
			}
		}
		(o(h, 'onerror'), wf(e, 'error', h));
		function S() {
			(e.removeListener('finish', b), R());
		}
		(o(S, 'onclose'), e.once('close', S));
		function b() {
			(w('onfinish'), e.removeListener('close', S), R());
		}
		(o(b, 'onfinish'), e.once('finish', b));
		function R() {
			(w('unpipe'), n.unpipe(e));
		}
		return (
			o(R, 'unpipe'),
			e.emit('pipe', n),
			e.writableNeedDrain === !0 ? c() : r.flowing || (w('pipe resume'), n.resume()),
			e
		);
	};
	function Hf(e, t) {
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
	o(Hf, 'pipeOnDrain');
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
		const r = ff(t.pipes, e);
		return r === -1
			? this
			: (t.pipes.splice(r, 1),
				t.pipes.length === 0 && this.pause(),
				e.emit('unpipe', this, n),
				this);
	};
	_.prototype.on = function (e, t) {
		const n = ee.prototype.on.call(this, e, t);
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
					r.length ? rt(this) : r.reading || j.nextTick(Gf, this)),
			n
		);
	};
	_.prototype.addListener = _.prototype.on;
	_.prototype.removeListener = function (e, t) {
		const n = ee.prototype.removeListener.call(this, e, t);
		return (e === 'readable' && j.nextTick(Qr, this), n);
	};
	_.prototype.off = _.prototype.removeListener;
	_.prototype.removeAllListeners = function (e) {
		const t = ee.prototype.removeAllListeners.apply(this, arguments);
		return ((e === 'readable' || e === void 0) && j.nextTick(Qr, this), t);
	};
	function Qr(e) {
		const t = e._readableState;
		((t.readableListening = e.listenerCount('readable') > 0),
		t.resumeScheduled && t[fe] === !1
			? (t.flowing = !0)
			: e.listenerCount('data') > 0
				? e.resume()
				: t.readableListening || (t.flowing = null));
	}
	o(Qr, 'updateReadableListening');
	function Gf(e) {
		(w('readable nexttick read 0'), e.read(0));
	}
	o(Gf, 'nReadingNextTick');
	_.prototype.resume = function () {
		const e = this._readableState;
		return (
			e.flowing || (w('resume'), (e.flowing = !e.readableListening), Vf(this, e)),
			(e[fe] = !1),
			this
		);
	};
	function Vf(e, t) {
		t.resumeScheduled || ((t.resumeScheduled = !0), j.nextTick(Yf, e, t));
	}
	o(Vf, 'resume');
	function Yf(e, t) {
		(w('resume', t.reading),
		t.reading || e.read(0),
		(t.resumeScheduled = !1),
		e.emit('resume'),
		Zr(e),
		t.flowing && !t.reading && e.read(0));
	}
	o(Yf, 'resume_');
	_.prototype.pause = function () {
		return (
			w('call pause flowing=%j', this._readableState.flowing),
			this._readableState.flowing !== !1
			&& (w('pause'), (this._readableState.flowing = !1), this.emit('pause')),
			(this._readableState[fe] = !0),
			this
		);
	};
	function Zr(e) {
		const t = e._readableState;
		for (w('flow', t.flowing); t.flowing && e.read() !== null;);
	}
	o(Zr, 'flow');
	_.prototype.wrap = function (e) {
		let t = !1;
		(e.on('data', (r) => {
			!this.push(r) && e.pause && ((t = !0), e.pause());
		}),
		e.on('end', () => {
			this.push(null);
		}),
		e.on('error', (r) => {
			Se(this, r);
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
		const n = cf(e);
		for (let r = 1; r < n.length; r++) {
			const i = n[r];
			this[i] === void 0 && typeof e[i] == 'function' && (this[i] = e[i].bind(e));
		}
		return this;
	};
	_.prototype[pf] = function () {
		return ei(this);
	};
	_.prototype.iterator = function (e) {
		return (e !== void 0 && kf(e, 'options'), ei(this, e));
	};
	function ei(e, t) {
		typeof e.read != 'function' && (e = _.wrap(e, { objectMode: !0 }));
		const n = Kf(e, t);
		return ((n.stream = e), n);
	}
	o(ei, 'streamToAsyncIterator');
	async function* Kf(e, t) {
		let n = Bt;
		function r(a) {
			this === e ? (n(), (n = Bt)) : (n = a);
		}
		(o(r, 'next'), e.on('readable', r));
		let i;
		const l = Hr(e, { writable: !1 }, (a) => {
			((i = a ? jr(i, a) : null), n(), (n = Bt));
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
					await new Ur(r);
				}
			}
		}
		catch (a) {
			throw ((i = jr(i, a)), i);
		}
		finally {
			(i || t?.destroyOnReturn !== !1) && (i === void 0 || e._readableState.autoDestroy)
				? Re.destroyer(e, null)
				: (e.off('readable', r), l());
		}
	}
	o(Kf, 'createAsyncIterator');
	Kt(_.prototype, {
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
	Kt(nt.prototype, {
		pipesCount: {
			__proto__: null,
			get() {
				return this.pipes.length;
			},
		},
		paused: {
			__proto__: null,
			get() {
				return this[fe] !== !1;
			},
			set(e) {
				this[fe] = !!e;
			},
		},
	});
	_._fromList = ti;
	function ti(e, t) {
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
	o(ti, 'fromList');
	function Ht(e) {
		const t = e._readableState;
		(w('endReadable', t.endEmitted), t.endEmitted || ((t.ended = !0), j.nextTick(zf, t, e)));
	}
	o(Ht, 'endReadable');
	function zf(e, t) {
		if (
			(w('endReadableNT', e.endEmitted, e.length),
			!e.errored && !e.closeEmitted && !e.endEmitted && e.length === 0)) {
			if (((e.endEmitted = !0), t.emit('end'), t.writable && t.allowHalfOpen === !1)) {
				j.nextTick(Xf, t);
			}
			else if (e.autoDestroy) {
				const n = t._writableState;
				(!n || (n.autoDestroy && (n.finished || n.writable === !1))) && t.destroy();
			}
		}
	}
	o(zf, 'endReadableNT');
	function Xf(e) {
		e.writable && !e.writableEnded && !e.destroyed && e.end();
	}
	o(Xf, 'endWritableNT');
	_.from = function (e, t) {
		return Nf(_, e, t);
	};
	let Gt;
	function ni() {
		return (Gt === void 0 && (Gt = {}), Gt);
	}
	o(ni, 'lazyWebStreams');
	_.fromWeb = function (e, t) {
		return ni().newStreamReadableFromReadableStream(e, t);
	};
	_.toWeb = function (e, t) {
		return ni().newReadableStreamFromStreamReadable(e, t);
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
				(Re.destroyer(e, i), l(i));
			},
		}).wrap(e);
	};
});
const ft = E((Ad, pi) => {
	'use strict';
	const ue = k('process');
	const {
		ArrayPrototypeSlice: li,
		Error: Jf,
		FunctionPrototypeSymbolHasInstance: ai,
		ObjectDefineProperty: fi,
		ObjectDefineProperties: Qf,
		ObjectSetPrototypeOf: ui,
		StringPrototypeToLowerCase: Zf,
		Symbol: eu,
		SymbolHasInstance: tu,
	} = T();
	pi.exports = I;
	I.WritableState = We;
	const { EventEmitter: nu } = k('events');
	const Pe = Xe().Stream;
	const { Buffer: it } = k('buffer');
	const at = ae();
	const { addAbortSignal: ru } = ke();
	const { getHighWaterMark: iu, getDefaultHighWaterMark: ou } = Ne();
	const {
		ERR_INVALID_ARG_TYPE: lu,
		ERR_METHOD_NOT_IMPLEMENTED: au,
		ERR_MULTIPLE_CALLBACK: si,
		ERR_STREAM_CANNOT_PIPE: fu,
		ERR_STREAM_DESTROYED: Oe,
		ERR_STREAM_ALREADY_FINISHED: uu,
		ERR_STREAM_NULL_VALUES: su,
		ERR_STREAM_WRITE_AFTER_END: du,
		ERR_UNKNOWN_ENCODING: di,
	} = L().codes;
	const { errorOrDestroy: me } = at;
	ui(I.prototype, Pe.prototype);
	ui(I, Pe);
	function Jt() {}
	o(Jt, 'nop');
	const Ae = eu('kOnFinished');
	function We(e, t, n) {
		(typeof n != 'boolean' && (n = t instanceof V()),
		(this.objectMode = !!(e && e.objectMode)),
		n && (this.objectMode = this.objectMode || !!(e && e.writableObjectMode)),
		(this.highWaterMark = e ? iu(this, e, 'writableHighWaterMark', n) : ou(!1)),
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
		(this.onwrite = hu.bind(void 0, t)),
		(this.writecb = null),
		(this.writelen = 0),
		(this.afterWriteTickInfo = null),
		lt(this),
		(this.pendingcb = 0),
		(this.constructed = !0),
		(this.prefinished = !1),
		(this.errorEmitted = !1),
		(this.emitClose = !e || e.emitClose !== !1),
		(this.autoDestroy = !e || e.autoDestroy !== !1),
		(this.errored = null),
		(this.closed = !1),
		(this.closeEmitted = !1),
		(this[Ae] = []));
	}
	o(We, 'WritableState');
	function lt(e) {
		((e.buffered = []), (e.bufferedIndex = 0), (e.allBuffers = !0), (e.allNoop = !0));
	}
	o(lt, 'resetBuffer');
	We.prototype.getBuffer = o(function () {
		return li(this.buffered, this.bufferedIndex);
	}, 'getBuffer');
	fi(We.prototype, 'bufferedRequestCount', {
		__proto__: null,
		get() {
			return this.buffered.length - this.bufferedIndex;
		},
	});
	function I(e) {
		const t = this instanceof V();
		if (!t && !ai(I, this))
			return new I(e);
		((this._writableState = new We(e, this, t)),
		e
		&& (typeof e.write == 'function' && (this._write = e.write),
		typeof e.writev == 'function' && (this._writev = e.writev),
		typeof e.destroy == 'function' && (this._destroy = e.destroy),
		typeof e.final == 'function' && (this._final = e.final),
		typeof e.construct == 'function' && (this._construct = e.construct),
		e.signal && ru(e.signal, this)),
		Pe.call(this, e),
		at.construct(this, () => {
			const n = this._writableState;
			(n.writing || Zt(this, n), en(this, n));
		}));
	}
	o(I, 'Writable');
	fi(I, tu, {
		__proto__: null,
		value: o(function (e) {
			return ai(this, e) ? !0 : this !== I ? !1 : e && e._writableState instanceof We;
		}, 'value'),
	});
	I.prototype.pipe = function () {
		me(this, new fu());
	};
	function ci(e, t, n, r) {
		const i = e._writableState;
		if (typeof n == 'function') {
			((r = n), (n = i.defaultEncoding));
		}
		else {
			if (!n)
				n = i.defaultEncoding;
			else if (n !== 'buffer' && !it.isEncoding(n))
				throw new di(n);
			typeof r != 'function' && (r = Jt);
		}
		if (t === null)
			throw new su();
		if (!i.objectMode) {
			if (typeof t == 'string')
				i.decodeStrings !== !1 && ((t = it.from(t, n)), (n = 'buffer'));
			else if (t instanceof it)
				n = 'buffer';
			else if (Pe._isUint8Array(t))
				((t = Pe._uint8ArrayToBuffer(t)), (n = 'buffer'));
			else throw new lu('chunk', ['string', 'Buffer', 'Uint8Array'], t);
		}
		let l;
		return (
			i.ending ? (l = new du()) : i.destroyed && (l = new Oe('write')),
			l ? (ue.nextTick(r, l), me(e, l, !0), l) : (i.pendingcb++, cu(e, i, t, n, r))
		);
	}
	o(ci, '_write');
	I.prototype.write = function (e, t, n) {
		return ci(this, e, t, n) === !0;
	};
	I.prototype.cork = function () {
		this._writableState.corked++;
	};
	I.prototype.uncork = function () {
		const e = this._writableState;
		e.corked && (e.corked--, e.writing || Zt(this, e));
	};
	I.prototype.setDefaultEncoding = o(function (t) {
		if ((typeof t == 'string' && (t = Zf(t)), !it.isEncoding(t)))
			throw new di(t);
		return ((this._writableState.defaultEncoding = t), this);
	}, 'setDefaultEncoding');
	function cu(e, t, n, r, i) {
		const l = t.objectMode ? 1 : n.length;
		t.length += l;
		const a = t.length < t.highWaterMark;
		return (
			a || (t.needDrain = !0),
			t.writing || t.corked || t.errored || !t.constructed
				? (t.buffered.push({ chunk: n, encoding: r, callback: i }),
					t.allBuffers && r !== 'buffer' && (t.allBuffers = !1),
					t.allNoop && i !== Jt && (t.allNoop = !1))
				: ((t.writelen = l),
					(t.writecb = i),
					(t.writing = !0),
					(t.sync = !0),
					e._write(n, r, t.onwrite),
					(t.sync = !1)),
			a && !t.errored && !t.destroyed
		);
	}
	o(cu, 'writeOrBuffer');
	function ii(e, t, n, r, i, l, a) {
		((t.writelen = r),
		(t.writecb = a),
		(t.writing = !0),
		(t.sync = !0),
		t.destroyed
			? t.onwrite(new Oe('write'))
			: n
				? e._writev(i, t.onwrite)
				: e._write(i, l, t.onwrite),
		(t.sync = !1));
	}
	o(ii, 'doWrite');
	function oi(e, t, n, r) {
		(--t.pendingcb, r(n), Qt(t), me(e, n));
	}
	o(oi, 'onwriteError');
	function hu(e, t) {
		const n = e._writableState;
		const r = n.sync;
		const i = n.writecb;
		if (typeof i != 'function') {
			me(e, new si());
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
				r ? ue.nextTick(oi, e, n, t, i) : oi(e, n, t, i))
			: (n.buffered.length > n.bufferedIndex && Zt(e, n),
				r
					? n.afterWriteTickInfo !== null && n.afterWriteTickInfo.cb === i
						? n.afterWriteTickInfo.count++
						: ((n.afterWriteTickInfo = { count: 1, cb: i, stream: e, state: n }),
							ue.nextTick(bu, n.afterWriteTickInfo))
					: hi(e, n, 1, i)));
	}
	o(hu, 'onwrite');
	function bu({ stream: e, state: t, count: n, cb: r }) {
		return ((t.afterWriteTickInfo = null), hi(e, t, n, r));
	}
	o(bu, 'afterWriteTick');
	function hi(e, t, n, r) {
		for (
			!t.ending
			&& !e.destroyed
			&& t.length === 0
			&& t.needDrain
			&& ((t.needDrain = !1), e.emit('drain'));
			n-- > 0;
		)
			(t.pendingcb--, r());
		(t.destroyed && Qt(t), en(e, t));
	}
	o(hi, 'afterWrite');
	function Qt(e) {
		if (e.writing)
			return;
		for (let i = e.bufferedIndex; i < e.buffered.length; ++i) {
			var t;
			const { chunk: l, callback: a } = e.buffered[i];
			const u = e.objectMode ? 1 : l.length;
			((e.length -= u), a((t = e.errored) !== null && t !== void 0 ? t : new Oe('write')));
		}
		const n = e[Ae].splice(0);
		for (let i = 0; i < n.length; i++) {
			var r;
			n[i]((r = e.errored) !== null && r !== void 0 ? r : new Oe('end'));
		}
		lt(e);
	}
	o(Qt, 'errorBuffer');
	function Zt(e, t) {
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
				? Jt
				: (f) => {
						for (let d = a; d < n.length; ++d) n[d].callback(f);
					};
			const s = t.allNoop && a === 0 ? n : li(n, a);
			((s.allBuffers = t.allBuffers), ii(e, t, !0, t.length, s, '', u), lt(t));
		}
		else {
			do {
				const { chunk: u, encoding: s, callback: f } = n[a];
				n[a++] = null;
				const d = i ? 1 : u.length;
				ii(e, t, !1, d, u, s, f);
			} while (a < n.length && !t.writing);
			a === n.length
				? lt(t)
				: a > 256
					? (n.splice(0, a), (t.bufferedIndex = 0))
					: (t.bufferedIndex = a);
		}
		t.bufferProcessing = !1;
	}
	o(Zt, 'clearBuffer');
	I.prototype._write = function (e, t, n) {
		if (this._writev)
			this._writev([{ chunk: e, encoding: t }], n);
		else throw new au('_write()');
	};
	I.prototype._writev = null;
	I.prototype.end = function (e, t, n) {
		const r = this._writableState;
		typeof e == 'function'
			? ((n = e), (e = null), (t = null))
			: typeof t == 'function' && ((n = t), (t = null));
		let i;
		if (e != null) {
			const l = ci(this, e, t);
			l instanceof Jf && (i = l);
		}
		return (
			r.corked && ((r.corked = 1), this.uncork()),
			i
			|| (!r.errored && !r.ending
				? ((r.ending = !0), en(this, r, !0), (r.ended = !0))
				: r.finished
					? (i = new uu('end'))
					: r.destroyed && (i = new Oe('end'))),
			typeof n == 'function' && (i || r.finished ? ue.nextTick(n, i) : r[Ae].push(n)),
			this
		);
	};
	function ot(e) {
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
	o(ot, 'needFinish');
	function pu(e, t) {
		let n = !1;
		function r(i) {
			if (n) {
				me(e, i ?? si());
				return;
			}
			if (((n = !0), t.pendingcb--, i)) {
				const l = t[Ae].splice(0);
				for (let a = 0; a < l.length; a++) l[a](i);
				me(e, i, t.sync);
			}
			else {
				ot(t) && ((t.prefinished = !0), e.emit('prefinish'), t.pendingcb++, ue.nextTick(Xt, e, t));
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
	o(pu, 'callFinal');
	function _u(e, t) {
		!t.prefinished
		&& !t.finalCalled
		&& (typeof e._final == 'function' && !t.destroyed
			? ((t.finalCalled = !0), pu(e, t))
			: ((t.prefinished = !0), e.emit('prefinish')));
	}
	o(_u, 'prefinish');
	function en(e, t, n) {
		ot(t)
		&& (_u(e, t),
		t.pendingcb === 0
		&& (n
			? (t.pendingcb++,
				ue.nextTick(
					(r, i) => {
						ot(i) ? Xt(r, i) : i.pendingcb--;
					},
					e,
					t,
				))
			: ot(t) && (t.pendingcb++, Xt(e, t))));
	}
	o(en, 'finishMaybe');
	function Xt(e, t) {
		(t.pendingcb--, (t.finished = !0));
		const n = t[Ae].splice(0);
		for (let r = 0; r < n.length; r++) n[r]();
		if ((e.emit('finish'), t.autoDestroy)) {
			const r = e._readableState;
			(!r || (r.autoDestroy && (r.endEmitted || r.readable === !1))) && e.destroy();
		}
	}
	o(Xt, 'finish');
	Qf(I.prototype, {
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
	const yu = at.destroy;
	I.prototype.destroy = function (e, t) {
		const n = this._writableState;
		return (
			!n.destroyed && (n.bufferedIndex < n.buffered.length || n[Ae].length) && ue.nextTick(Qt, n),
			yu.call(this, e, t),
			this
		);
	};
	I.prototype._undestroy = at.undestroy;
	I.prototype._destroy = function (e, t) {
		t(e);
	};
	I.prototype[nu.captureRejectionSymbol] = function (e) {
		this.destroy(e);
	};
	let zt;
	function bi() {
		return (zt === void 0 && (zt = {}), zt);
	}
	o(bi, 'lazyWebStreams');
	I.fromWeb = function (e, t) {
		return bi().newStreamWritableFromWritableStream(e, t);
	};
	I.toWeb = function (e) {
		return bi().newWritableStreamFromStreamWritable(e);
	};
});
const Ni = E((Id, ki) => {
	const tn = k('process');
	const wu = k('buffer');
	const {
		isReadable: gu,
		isWritable: Su,
		isIterable: _i,
		isNodeStream: Eu,
		isReadableNodeStream: yi,
		isWritableNodeStream: wi,
		isDuplexNodeStream: Ru,
		isReadableStream: gi,
		isWritableStream: Si,
	} = U();
	const Ei = H();
	const {
		AbortError: Mi,
		codes: { ERR_INVALID_ARG_TYPE: mu, ERR_INVALID_RETURN_VALUE: Ri },
	} = L();
	const { destroyer: Ie } = ae();
	const Au = V();
	const qi = Le();
	const Tu = ft();
	const { createDeferredPromise: mi } = O();
	const Ai = jt();
	const Ti = globalThis.Blob || wu.Blob;
	const Iu = o(
		typeof Ti < 'u'
			? (t) => {
					return t instanceof Ti;
				}
			: (t) => {
					return !1;
				},
		'isBlob',
	);
	const Mu = globalThis.AbortController || _e().AbortController;
	const { FunctionPrototypeCall: Ii } = T();
	const te = class extends Au {
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
	ki.exports = o(function e(t, n) {
		if (Ru(t))
			return t;
		if (yi(t))
			return Te({ readable: t });
		if (wi(t))
			return Te({ writable: t });
		if (Eu(t))
			return Te({ writable: !1, readable: !1 });
		if (gi(t))
			return Te({ readable: qi.fromWeb(t) });
		if (Si(t))
			return Te({ writable: Tu.fromWeb(t) });
		if (typeof t == 'function') {
			const { value: i, write: l, final: a, destroy: u } = qu(t);
			if (_i(i))
				return Ai(te, i, { objectMode: !0, write: l, final: a, destroy: u });
			const s = i?.then;
			if (typeof s == 'function') {
				let f;
				const d = Ii(
					s,
					i,
					(c) => {
						if (c != null)
							throw new Ri('nully', 'body', c);
					},
					(c) => {
						Ie(f, c);
					},
				);
				return (f = new te({
					objectMode: !0,
					readable: !1,
					write: l,
					final(c) {
						a(async () => {
							try {
								(await d, tn.nextTick(c, null));
							}
							catch (p) {
								tn.nextTick(c, p);
							}
						});
					},
					destroy: u,
				}));
			}
			throw new Ri('Iterable, AsyncIterable or AsyncFunction', n, i);
		}
		if (Iu(t))
			return e(t.arrayBuffer());
		if (_i(t))
			return Ai(te, t, { objectMode: !0, writable: !1 });
		if (gi(t?.readable) && Si(t?.writable))
			return te.fromWeb(t);
		if (typeof t?.writable == 'object' || typeof t?.readable == 'object') {
			const i = t != null && t.readable ? (yi(t?.readable) ? t?.readable : e(t.readable)) : void 0;
			const l = t != null && t.writable ? (wi(t?.writable) ? t?.writable : e(t.writable)) : void 0;
			return Te({ readable: i, writable: l });
		}
		const r = t?.then;
		if (typeof r == 'function') {
			let i;
			return (
				Ii(
					r,
					t,
					(l) => {
						(l != null && i.push(l), i.push(null));
					},
					(l) => {
						Ie(i, l);
					},
				),
				(i = new te({ objectMode: !0, writable: !1, read() {} }))
			);
		}
		throw new mu(
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
	function qu(e) {
		let { promise: t, resolve: n } = mi();
		const r = new Mu();
		const i = r.signal;
		return {
			value: e(
				(async function* () {
					for (;;) {
						const a = t;
						t = null;
						const { chunk: u, done: s, cb: f } = await a;
						if ((tn.nextTick(f), s))
							return;
						if (i.aborted)
							throw new Mi(void 0, { cause: i.reason });
						(({ promise: t, resolve: n } = mi()), yield u);
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
	o(qu, 'fromAsyncGen');
	function Te(e) {
		const t = e.readable && typeof e.readable.read != 'function' ? qi.wrap(e.readable) : e.readable;
		const n = e.writable;
		let r = !!gu(t);
		let i = !!Su(n);
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
			(f = new te({
				readableObjectMode: !!(t != null && t.readableObjectMode),
				writableObjectMode: !!(n != null && n.writableObjectMode),
				readable: r,
				writable: i,
			})),
			i
			&& (Ei(n, (c) => {
				((i = !1), c && Ie(t, c), d(c));
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
			&& (Ei(t, (c) => {
				((r = !1), c && Ie(t, c), d(c));
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
				(!c && s !== null && (c = new Mi()),
				(u = null),
				(l = null),
				(a = null),
				s === null ? p(c) : ((s = p), Ie(n, c), Ie(t, c)));
			}),
			f
		);
	}
	o(Te, '_duplexify');
});
var V = E((qd, Pi) => {
	'use strict';
	const {
		ObjectDefineProperties: ku,
		ObjectGetOwnPropertyDescriptor: z,
		ObjectKeys: Nu,
		ObjectSetPrototypeOf: Di,
	} = T();
	Pi.exports = F;
	const on = Le();
	const v = ft();
	Di(F.prototype, on.prototype);
	Di(F, on);
	{
		const e = Nu(v.prototype);
		for (let t = 0; t < e.length; t++) {
			const n = e[t];
			F.prototype[n] || (F.prototype[n] = v.prototype[n]);
		}
	}
	function F(e) {
		if (!(this instanceof F))
			return new F(e);
		(on.call(this, e),
		v.call(this, e),
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
	ku(F.prototype, {
		writable: { __proto__: null, ...z(v.prototype, 'writable') },
		writableHighWaterMark: { __proto__: null, ...z(v.prototype, 'writableHighWaterMark') },
		writableObjectMode: { __proto__: null, ...z(v.prototype, 'writableObjectMode') },
		writableBuffer: { __proto__: null, ...z(v.prototype, 'writableBuffer') },
		writableLength: { __proto__: null, ...z(v.prototype, 'writableLength') },
		writableFinished: { __proto__: null, ...z(v.prototype, 'writableFinished') },
		writableCorked: { __proto__: null, ...z(v.prototype, 'writableCorked') },
		writableEnded: { __proto__: null, ...z(v.prototype, 'writableEnded') },
		writableNeedDrain: { __proto__: null, ...z(v.prototype, 'writableNeedDrain') },
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
	let nn;
	function Li() {
		return (nn === void 0 && (nn = {}), nn);
	}
	o(Li, 'lazyWebStreams');
	F.fromWeb = function (e, t) {
		return Li().newStreamDuplexFromReadableWritablePair(e, t);
	};
	F.toWeb = function (e) {
		return Li().newReadableWritablePairFromDuplex(e);
	};
	let rn;
	F.from = function (e) {
		return (rn || (rn = Ni()), rn(e, 'body'));
	};
});
const fn = E((Nd, Wi) => {
	'use strict';
	const { ObjectSetPrototypeOf: Oi, Symbol: Du } = T();
	Wi.exports = X;
	const { ERR_METHOD_NOT_IMPLEMENTED: Lu } = L().codes;
	const an = V();
	const { getHighWaterMark: Pu } = Ne();
	Oi(X.prototype, an.prototype);
	Oi(X, an);
	const xe = Du('kCallback');
	function X(e) {
		if (!(this instanceof X))
			return new X(e);
		const t = e ? Pu(this, e, 'readableHighWaterMark', !0) : null;
		(t === 0
			&& (e = {
				...e,
				highWaterMark: null,
				readableHighWaterMark: t,
				writableHighWaterMark: e.writableHighWaterMark || 0,
			}),
		an.call(this, e),
		(this._readableState.sync = !1),
		(this[xe] = null),
		e
		&& (typeof e.transform == 'function' && (this._transform = e.transform),
		typeof e.flush == 'function' && (this._flush = e.flush)),
		this.on('prefinish', Ou));
	}
	o(X, 'Transform');
	function ln(e) {
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
	o(ln, 'final');
	function Ou() {
		this._final !== ln && ln.call(this);
	}
	o(Ou, 'prefinish');
	X.prototype._final = ln;
	X.prototype._transform = function (e, t, n) {
		throw new Lu('_transform()');
	};
	X.prototype._write = function (e, t, n) {
		const r = this._readableState;
		const i = this._writableState;
		const l = r.length;
		this._transform(e, t, (a, u) => {
			if (a) {
				n(a);
				return;
			}
			(u != null && this.push(u),
			i.ended || l === r.length || r.length < r.highWaterMark ? n() : (this[xe] = n));
		});
	};
	X.prototype._read = function () {
		if (this[xe]) {
			const e = this[xe];
			((this[xe] = null), e());
		}
	};
});
const sn = E((Ld, Ci) => {
	'use strict';
	const { ObjectSetPrototypeOf: xi } = T();
	Ci.exports = Me;
	const un = fn();
	xi(Me.prototype, un.prototype);
	xi(Me, un);
	function Me(e) {
		if (!(this instanceof Me))
			return new Me(e);
		un.call(this, e);
	}
	o(Me, 'PassThrough');
	Me.prototype._transform = function (e, t, n) {
		n(null, e);
	};
});
const ve = E((Od, Bi) => {
	const Ce = k('process');
	const { ArrayIsArray: Wu, Promise: xu, SymbolAsyncIterator: Cu, SymbolDispose: vu } = T();
	const dt = H();
	const { once: $u } = O();
	const ju = ae();
	const vi = V();
	const {
		aggregateTwoErrors: Fu,
		codes: {
			ERR_INVALID_ARG_TYPE: gn,
			ERR_INVALID_RETURN_VALUE: dn,
			ERR_MISSING_ARGS: Bu,
			ERR_STREAM_DESTROYED: Uu,
			ERR_STREAM_PREMATURE_CLOSE: Hu,
		},
		AbortError: Gu,
	} = L();
	const { validateFunction: Vu, validateAbortSignal: Yu } = we();
	const {
		isIterable: se,
		isReadable: cn,
		isReadableNodeStream: st,
		isNodeStream: $i,
		isTransformStream: qe,
		isWebStream: Ku,
		isReadableStream: hn,
		isReadableFinished: zu,
	} = U();
	const Xu = globalThis.AbortController || _e().AbortController;
	let bn;
	let pn;
	let _n;
	function ji(e, t, n) {
		let r = !1;
		e.on('close', () => {
			r = !0;
		});
		const i = dt(e, { readable: t, writable: n }, (l) => {
			r = !l;
		});
		return {
			destroy: o((l) => {
				r || ((r = !0), ju.destroyer(e, l || new Uu('pipe')));
			}, 'destroy'),
			cleanup: i,
		};
	}
	o(ji, 'destroyer');
	function Ju(e) {
		return (Vu(e.at(-1), 'streams[stream.length - 1]'), e.pop());
	}
	o(Ju, 'popCallback');
	function yn(e) {
		if (se(e))
			return e;
		if (st(e))
			return Qu(e);
		throw new gn('val', ['Readable', 'Iterable', 'AsyncIterable'], e);
	}
	o(yn, 'makeAsyncIterable');
	async function* Qu(e) {
		(pn || (pn = Le()), yield* pn.prototype[Cu].call(e));
	}
	o(Qu, 'fromReadable');
	async function ut(e, t, n, { end: r }) {
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
				new xu((f, d) => {
					i
						? d(i)
						: (l = o(() => {
								i ? d(i) : f();
							}, 'onresolve'));
				}),
			'wait',
		);
		t.on('drain', a);
		const s = dt(t, { readable: !1 }, a);
		try {
			t.writableNeedDrain && (await u());
			for await (const f of e) t.write(f) || (await u());
			(r && (t.end(), await u()), n());
		}
		catch (f) {
			n(i !== f ? Fu(i, f) : f);
		}
		finally {
			(s(), t.off('drain', a));
		}
	}
	o(ut, 'pumpToNode');
	async function wn(e, t, n, { end: r }) {
		qe(t) && (t = t.writable);
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
	o(wn, 'pumpToWeb');
	function Zu(...e) {
		return Fi(e, $u(Ju(e)));
	}
	o(Zu, 'pipeline');
	function Fi(e, t, n) {
		if ((e.length === 1 && Wu(e[0]) && (e = e[0]), e.length < 2))
			throw new Bu('streams');
		const r = new Xu();
		const i = r.signal;
		const l = n?.signal;
		const a = [];
		Yu(l, 'options.signal');
		function u() {
			S(new Gu());
		}
		(o(u, 'abort'), (_n = _n || O().addAbortListener));
		let s;
		l && (s = _n(l, u));
		let f;
		let d;
		const c = [];
		let p = 0;
		function h(q) {
			S(q, --p === 0);
		}
		o(h, 'finish');
		function S(q, g) {
			let M;
			if ((q && (!f || f.code === 'ERR_STREAM_PREMATURE_CLOSE') && (f = q), !(!f && !g))) {
				for (; c.length;) c.shift()(f);
				((M = s) === null || M === void 0 || M[vu](),
				r.abort(),
				g && (f || a.forEach(ne => ne()), Ce.nextTick(t, f, d)));
			}
		}
		o(S, 'finishImpl');
		let b;
		for (let q = 0; q < e.length; q++) {
			const g = e[q];
			const M = q < e.length - 1;
			const ne = q > 0;
			const x = M || n?.end !== !1;
			const pe = q === e.length - 1;
			if ($i(g)) {
				const W = function (K) {
					K && K.name !== 'AbortError' && K.code !== 'ERR_STREAM_PREMATURE_CLOSE' && h(K);
				};
				const m = W;
				if ((o(W, 'onError'), x)) {
					const { destroy: K, cleanup: _t } = ji(g, M, ne);
					(c.push(K), cn(g) && pe && a.push(_t));
				}
				(g.on('error', W),
				cn(g)
				&& pe
				&& a.push(() => {
					g.removeListener('error', W);
				}));
			}
			if (q === 0) {
				if (typeof g == 'function') {
					if (((b = g({ signal: i })), !se(b)))
						throw new dn('Iterable, AsyncIterable or Stream', 'source', b);
				}
				else {
					se(g) || st(g) || qe(g) ? (b = g) : (b = vi.from(g));
				}
			}
			else if (typeof g == 'function') {
				if (qe(b)) {
					var R;
					b = yn((R = b) === null || R === void 0 ? void 0 : R.readable);
				}
				else {
					b = yn(b);
				}
				if (((b = g(b, { signal: i })), M)) {
					if (!se(b, !0))
						throw new dn('AsyncIterable', `transform[${q - 1}]`, b);
				}
				else {
					var y;
					bn || (bn = sn());
					const W = new bn({ objectMode: !0 });
					const K = (y = b) === null || y === void 0 ? void 0 : y.then;
					if (typeof K == 'function') {
						(p++,
						K.call(
							b,
							(Q) => {
								((d = Q), Q != null && W.write(Q), x && W.end(), Ce.nextTick(h));
							},
							(Q) => {
								(W.destroy(Q), Ce.nextTick(h, Q));
							},
						));
					}
					else if (se(b, !0)) {
						(p++, ut(b, W, h, { end: x }));
					}
					else if (hn(b) || qe(b)) {
						const Q = b.readable || b;
						(p++, ut(Q, W, h, { end: x }));
					}
					else {
						throw new dn('AsyncIterable or Promise', 'destination', b);
					}
					b = W;
					const { destroy: _t, cleanup: Wo } = ji(b, !1, !0);
					(c.push(_t), pe && a.push(Wo));
				}
			}
			else if ($i(g)) {
				if (st(b)) {
					p += 2;
					const W = es(b, g, h, { end: x });
					cn(g) && pe && a.push(W);
				}
				else if (qe(b) || hn(b)) {
					const W = b.readable || b;
					(p++, ut(W, g, h, { end: x }));
				}
				else if (se(b)) {
					(p++, ut(b, g, h, { end: x }));
				}
				else {
					throw new gn(
						'val',
						['Readable', 'Iterable', 'AsyncIterable', 'ReadableStream', 'TransformStream'],
						b,
					);
				}
				b = g;
			}
			else if (Ku(g)) {
				if (st(b)) {
					(p++, wn(yn(b), g, h, { end: x }));
				}
				else if (hn(b) || se(b)) {
					(p++, wn(b, g, h, { end: x }));
				}
				else if (qe(b)) {
					(p++, wn(b.readable, g, h, { end: x }));
				}
				else {
					throw new gn(
						'val',
						['Readable', 'Iterable', 'AsyncIterable', 'ReadableStream', 'TransformStream'],
						b,
					);
				}
				b = g;
			}
			else {
				b = vi.from(g);
			}
		}
		return (((i != null && i.aborted) || (l != null && l.aborted)) && Ce.nextTick(u), b);
	}
	o(Fi, 'pipelineImpl');
	function es(e, t, n, { end: r }) {
		let i = !1;
		if (
			(t.on('close', () => {
				i || n(new Hu());
			}),
			e.pipe(t, { end: !1 }),
			r)) {
			const a = function () {
				((i = !0), t.end());
			};
			const l = a;
			(o(a, 'endFn'), zu(e) ? Ce.nextTick(a) : e.once('end', a));
		}
		else {
			n();
		}
		return (
			dt(e, { readable: !0, writable: !1 }, (a) => {
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
			dt(t, { readable: !1, writable: !0 }, n)
		);
	}
	o(es, 'pipe');
	Bi.exports = { pipelineImpl: Fi, pipeline: Zu };
});
const En = E((xd, Ki) => {
	'use strict';
	const { pipeline: ts } = ve();
	const ct = V();
	const { destroyer: ns } = ae();
	const {
		isNodeStream: ht,
		isReadable: Ui,
		isWritable: Hi,
		isWebStream: Sn,
		isTransformStream: de,
		isWritableStream: Gi,
		isReadableStream: Vi,
	} = U();
	const {
		AbortError: rs,
		codes: { ERR_INVALID_ARG_VALUE: Yi, ERR_MISSING_ARGS: is },
	} = L();
	const os = H();
	Ki.exports = o((...t) => {
		if (t.length === 0)
			throw new is('streams');
		if (t.length === 1)
			return ct.from(t[0]);
		const n = [...t];
		if (
			(typeof t[0] == 'function' && (t[0] = ct.from(t[0])), typeof t.at(-1) == 'function')
		) {
			const h = t.length - 1;
			t[h] = ct.from(t[h]);
		}
		for (let h = 0; h < t.length; ++h) {
			if (!(!ht(t[h]) && !Sn(t[h]))) {
				if (h < t.length - 1 && !(Ui(t[h]) || Vi(t[h]) || de(t[h])))
					throw new Yi(`streams[${h}]`, n[h], 'must be readable');
				if (h > 0 && !(Hi(t[h]) || Gi(t[h]) || de(t[h])))
					throw new Yi(`streams[${h}]`, n[h], 'must be writable');
			}
		}
		let r, i, l, a, u;
		function s(h) {
			const S = a;
			((a = null), S ? S(h) : h ? u.destroy(h) : !p && !c && u.destroy());
		}
		o(s, 'onfinished');
		const f = t[0];
		const d = ts(t, s);
		let c = !!(Hi(f) || Gi(f) || de(f));
		let p = !!(Ui(d) || Vi(d) || de(d));
		if (
			((u = new ct({
				writableObjectMode: !!(f != null && f.writableObjectMode),
				readableObjectMode: !!(d != null && d.readableObjectMode),
				writable: c,
				readable: p,
			})),
			c)) {
			if (ht(f)) {
				((u._write = function (S, b, R) {
					f.write(S, b) ? R() : (r = R);
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
			else if (Sn(f)) {
				const b = (de(f) ? f.writable : f).getWriter();
				((u._write = async function (R, y, m) {
					try {
						(await b.ready, b.write(R).catch(() => {}), m());
					}
					catch (q) {
						m(q);
					}
				}),
				(u._final = async function (R) {
					try {
						(await b.ready, b.close().catch(() => {}), (i = R));
					}
					catch (y) {
						R(y);
					}
				}));
			}
			const h = de(d) ? d.readable : d;
			os(h, () => {
				if (i) {
					const S = i;
					((i = null), S());
				}
			});
		}
		if (p) {
			if (ht(d)) {
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
			else if (Sn(d)) {
				const S = (de(d) ? d.readable : d).getReader();
				u._read = async function () {
					for (;;) {
						try {
							const { value: b, done: R } = await S.read();
							if (!u.push(b))
								return;
							if (R) {
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
				(!h && a !== null && (h = new rs()),
				(l = null),
				(r = null),
				(i = null),
				a === null ? S(h) : ((a = S), ht(d) && ns(d, h)));
			}),
			u
		);
	}, 'compose');
});
const io = E((vd, An) => {
	'use strict';
	const ls = globalThis.AbortController || _e().AbortController;
	const {
		codes: {
			ERR_INVALID_ARG_VALUE: as,
			ERR_INVALID_ARG_TYPE: $e,
			ERR_MISSING_ARGS: fs,
			ERR_OUT_OF_RANGE: us,
		},
		AbortError: Y,
	} = L();
	const { validateAbortSignal: ce, validateInteger: zi, validateObject: he } = we();
	const ss = T().Symbol('kWeak');
	const ds = T().Symbol('kResistStopPropagation');
	const { finished: cs } = H();
	const hs = En();
	const { addAbortSignalNoValidate: bs } = ke();
	const { isWritable: ps, isNodeStream: _s } = U();
	const { deprecate: ys } = O();
	const {
		ArrayPrototypePush: ws,
		Boolean: gs,
		MathFloor: Xi,
		Number: Ss,
		NumberIsNaN: Es,
		Promise: Ji,
		PromiseReject: Qi,
		PromiseResolve: Rs,
		PromisePrototypeThen: Zi,
		Symbol: to,
	} = T();
	const bt = to('kEmpty');
	const eo = to('kEof');
	function ms(e, t) {
		if (
			(t != null && he(t, 'options'),
			t?.signal != null && ce(t.signal, 'options.signal'),
			_s(e) && !ps(e))) {
			throw new as('stream', e, 'must be writable');
		}
		const n = hs(this, e);
		return (t != null && t.signal && bs(t.signal, n), n);
	}
	o(ms, 'compose');
	function pt(e, t) {
		if (typeof e != 'function')
			throw new $e('fn', ['Function', 'AsyncFunction'], e);
		(t != null && he(t, 'options'), t?.signal != null && ce(t.signal, 'options.signal'));
		let n = 1;
		t?.concurrency != null && (n = Xi(t.concurrency));
		let r = n - 1;
		return (
			t?.highWaterMark != null && (r = Xi(t.highWaterMark)),
			zi(n, 'options.concurrency', 1),
			zi(r, 'options.highWaterMark', 0),
			(r += n),
			o(async function* () {
				const l = O().AbortSignalAny([t?.signal].filter(gs));
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
				async function R() {
					try {
						for await (let y of a) {
							if (c)
								return;
							if (l.aborted)
								throw new Y();
							try {
								if (((y = e(y, s)), y === bt))
									continue;
								y = Rs(y);
							}
							catch (m) {
								y = Qi(m);
							}
							((p += 1),
							Zi(y, S, h),
							u.push(y),
							f && (f(), (f = null)),
							!c
							&& (u.length >= r || p >= n)
							&& (await new Ji((m) => {
								d = m;
							})));
						}
						u.push(eo);
					}
					catch (y) {
						const m = Qi(y);
						(Zi(m, S, h), u.push(m));
					}
					finally {
						((c = !0), f && (f(), (f = null)));
					}
				}
				(o(R, 'pump'), R());
				try {
					for (;;) {
						for (; u.length > 0;) {
							const y = await u[0];
							if (y === eo)
								return;
							if (l.aborted)
								throw new Y();
							(y !== bt && (yield y), u.shift(), b());
						}
						await new Ji((y) => {
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
	o(pt, 'map');
	function As(e = void 0) {
		return (
			e != null && he(e, 'options'),
			e?.signal != null && ce(e.signal, 'options.signal'),
			o(async function* () {
				let n = 0;
				for await (const i of this) {
					var r;
					if (e != null && (r = e.signal) !== null && r !== void 0 && r.aborted)
						throw new Y({ cause: e.signal.reason });
					yield [n++, i];
				}
			}, 'asIndexedPairs').call(this)
		);
	}
	o(As, 'asIndexedPairs');
	async function no(e, t = void 0) {
		for await (const n of mn.call(this, e, t)) return !0;
		return !1;
	}
	o(no, 'some');
	async function Ts(e, t = void 0) {
		if (typeof e != 'function')
			throw new $e('fn', ['Function', 'AsyncFunction'], e);
		return !(await no.call(this, async (...n) => !(await e(...n)), t));
	}
	o(Ts, 'every');
	async function Is(e, t) {
		for await (const n of mn.call(this, e, t)) return n;
	}
	o(Is, 'find');
	async function Ms(e, t) {
		if (typeof e != 'function')
			throw new $e('fn', ['Function', 'AsyncFunction'], e);
		async function n(r, i) {
			return (await e(r, i), bt);
		}
		o(n, 'forEachFn');
		for await (const r of pt.call(this, n, t));
	}
	o(Ms, 'forEach');
	function mn(e, t) {
		if (typeof e != 'function')
			throw new $e('fn', ['Function', 'AsyncFunction'], e);
		async function n(r, i) {
			return (await e(r, i)) ? r : bt;
		}
		return (o(n, 'filterFn'), pt.call(this, n, t));
	}
	o(mn, 'filter');
	const Rn = class extends fs {
		static {
			o(this, 'ReduceAwareErrMissingArgs');
		}

		constructor() {
			(super('reduce'), (this.message = 'Reduce of an empty stream requires an initial value'));
		}
	};
	async function qs(e, t, n) {
		let r;
		if (typeof e != 'function')
			throw new $e('reducer', ['Function', 'AsyncFunction'], e);
		(n != null && he(n, 'options'), n?.signal != null && ce(n.signal, 'options.signal'));
		let i = arguments.length > 1;
		if (n != null && (r = n.signal) !== null && r !== void 0 && r.aborted) {
			const f = new Y(void 0, { cause: n.signal.reason });
			throw (this.once('error', () => {}), await cs(this.destroy(f)), f);
		}
		const l = new ls();
		const a = l.signal;
		if (n != null && n.signal) {
			const f = { once: !0, [ss]: this, [ds]: !0 };
			n.signal.addEventListener('abort', () => l.abort(), f);
		}
		let u = !1;
		try {
			for await (const f of this) {
				var s;
				if (((u = !0), n != null && (s = n.signal) !== null && s !== void 0 && s.aborted))
					throw new Y();
				i ? (t = await e(t, f, { signal: a })) : ((t = f), (i = !0));
			}
			if (!u && !i)
				throw new Rn();
		}
		finally {
			l.abort();
		}
		return t;
	}
	o(qs, 'reduce');
	async function ks(e) {
		(e != null && he(e, 'options'), e?.signal != null && ce(e.signal, 'options.signal'));
		const t = [];
		for await (const r of this) {
			var n;
			if (e != null && (n = e.signal) !== null && n !== void 0 && n.aborted)
				throw new Y(void 0, { cause: e.signal.reason });
			ws(t, r);
		}
		return t;
	}
	o(ks, 'toArray');
	function Ns(e, t) {
		const n = pt.call(this, e, t);
		return o(async function* () {
			for await (const i of n) yield* i;
		}, 'flatMap').call(this);
	}
	o(Ns, 'flatMap');
	function ro(e) {
		if (((e = Ss(e)), Es(e)))
			return 0;
		if (e < 0)
			throw new us('number', '>= 0', e);
		return e;
	}
	o(ro, 'toIntegerOrInfinity');
	function Ds(e, t = void 0) {
		return (
			t != null && he(t, 'options'),
			t?.signal != null && ce(t.signal, 'options.signal'),
			(e = ro(e)),
			o(async function* () {
				let r;
				if (t != null && (r = t.signal) !== null && r !== void 0 && r.aborted)
					throw new Y();
				for await (const l of this) {
					var i;
					if (t != null && (i = t.signal) !== null && i !== void 0 && i.aborted)
						throw new Y();
					e-- <= 0 && (yield l);
				}
			}, 'drop').call(this)
		);
	}
	o(Ds, 'drop');
	function Ls(e, t = void 0) {
		return (
			t != null && he(t, 'options'),
			t?.signal != null && ce(t.signal, 'options.signal'),
			(e = ro(e)),
			o(async function* () {
				let r;
				if (t != null && (r = t.signal) !== null && r !== void 0 && r.aborted)
					throw new Y();
				for await (const l of this) {
					var i;
					if (t != null && (i = t.signal) !== null && i !== void 0 && i.aborted)
						throw new Y();
					if ((e-- > 0 && (yield l), e <= 0))
						return;
				}
			}, 'take').call(this)
		);
	}
	o(Ls, 'take');
	An.exports.streamReturningOperators = {
		asIndexedPairs: ys(As, 'readable.asIndexedPairs will be removed in a future version.'),
		drop: Ds,
		filter: mn,
		flatMap: Ns,
		map: pt,
		take: Ls,
		compose: ms,
	};
	An.exports.promiseReturningOperators = {
		every: Ts,
		forEach: Ms,
		reduce: qs,
		toArray: ks,
		some: no,
		find: Is,
	};
});
const lo = E((jd, oo) => {
	'use strict';
	const { ArrayPrototypePop: Ps, Promise: Os } = T();
	const { isIterable: Ws, isNodeStream: xs, isWebStream: Cs } = U();
	const { pipelineImpl: vs } = ve();
	const { finished: $s } = H();
	be();
	function js(...e) {
		return new Os((t, n) => {
			let r;
			let i;
			const l = e.at(-1);
			if (l && typeof l == 'object' && !xs(l) && !Ws(l) && !Cs(l)) {
				const a = Ps(e);
				((r = a.signal), (i = a.end));
			}
			vs(
				e,
				(a, u) => {
					a ? n(a) : t(u);
				},
				{ signal: r, end: i },
			);
		});
	}
	o(js, 'pipeline');
	oo.exports = { finished: $s, pipeline: js };
});
var be = E((Bd, _o) => {
	'use strict';
	const { Buffer: Fs } = k('buffer');
	const { ObjectDefineProperty: J, ObjectKeys: uo, ReflectApply: so } = T();
	const {
		promisify: { custom: co },
	} = O();
	const { streamReturningOperators: ao, promiseReturningOperators: fo } = io();
	const {
		codes: { ERR_ILLEGAL_CONSTRUCTOR: ho },
	} = L();
	const Bs = En();
	const { setDefaultHighWaterMark: Us, getDefaultHighWaterMark: Hs } = Ne();
	const { pipeline: bo } = ve();
	const { destroyer: Gs } = ae();
	const po = H();
	const Tn = lo();
	const je = U();
	const A = (_o.exports = Xe().Stream);
	A.isDestroyed = je.isDestroyed;
	A.isDisturbed = je.isDisturbed;
	A.isErrored = je.isErrored;
	A.isReadable = je.isReadable;
	A.isWritable = je.isWritable;
	A.Readable = Le();
	for (const e of uo(ao)) {
		const n = function (...r) {
			if (new.target)
				throw ho();
			return A.Readable.from(so(t, this, r));
		};
		o(n, 'fn');
		let t = ao[e];
		(J(n, 'name', { __proto__: null, value: t.name }),
		J(n, 'length', { __proto__: null, value: t.length }),
		J(A.Readable.prototype, e, {
			__proto__: null,
			value: n,
			enumerable: !1,
			configurable: !0,
			writable: !0,
		}));
	}
	for (const e of uo(fo)) {
		const n = function (...r) {
			if (new.target)
				throw ho();
			return so(t, this, r);
		};
		o(n, 'fn');
		let t = fo[e];
		(J(n, 'name', { __proto__: null, value: t.name }),
		J(n, 'length', { __proto__: null, value: t.length }),
		J(A.Readable.prototype, e, {
			__proto__: null,
			value: n,
			enumerable: !1,
			configurable: !0,
			writable: !0,
		}));
	}
	A.Writable = ft();
	A.Duplex = V();
	A.Transform = fn();
	A.PassThrough = sn();
	A.pipeline = bo;
	const { addAbortSignal: Vs } = ke();
	A.addAbortSignal = Vs;
	A.finished = po;
	A.destroy = Gs;
	A.compose = Bs;
	A.setDefaultHighWaterMark = Us;
	A.getDefaultHighWaterMark = Hs;
	J(A, 'promises', {
		__proto__: null,
		configurable: !0,
		enumerable: !0,
		get() {
			return Tn;
		},
	});
	J(bo, co, {
		__proto__: null,
		enumerable: !0,
		get() {
			return Tn.pipeline;
		},
	});
	J(po, co, {
		__proto__: null,
		enumerable: !0,
		get() {
			return Tn.finished;
		},
	});
	A.Stream = A;
	A._isUint8Array = o((t) => {
		return t instanceof Uint8Array;
	}, 'isUint8Array');
	A._uint8ArrayToBuffer = o((t) => {
		return Fs.from(t.buffer, t.byteOffset, t.byteLength);
	}, '_uint8ArrayToBuffer');
});
const wo = E((Hd, yo) => {
	'use strict';
	yo.exports = be().Readable;
});
const So = E((Gd, go) => {
	'use strict';
	go.exports = be().Writable;
});
const Ro = E((Vd, Eo) => {
	'use strict';
	Eo.exports = be().Duplex;
});
const Ao = E((Yd, mo) => {
	'use strict';
	mo.exports = be().Transform;
});
const Io = E((Kd, To) => {
	'use strict';
	To.exports = be().PassThrough;
});
const qo = re(wo(), 1);
const ko = re(So(), 1);
const No = re(Ro(), 1);
const Do = re(Ao(), 1);
const Lo = re(Io(), 1);
const Po = re(H(), 1);
const Oo = re(ve(), 1);

Ys($, Mo);
$.Readable = qo.Readable;
$.Writable = ko.Writable;
$.Duplex = No.Duplex;
$.Transform = Do.Transform;
$.PassThrough = Lo.PassThrough;
$.finished = Po.finished;
$.pipeline = Oo.pipeline;
$.Stream = $;
function $() {
	Mo.call(this);
}
o($, 'Stream');
$.prototype.pipe = function (e, t) {
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
		if ((f(), n.listenerCount('error') === 0))
			throw d;
	}
	(o(s, 'onerror'), n.on('error', s), e.on('error', s));
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
const Jd = $;
const export_Duplex = No.Duplex;
const export_PassThrough = Lo.PassThrough;
const export_Readable = qo.Readable;
const export_Transform = Do.Transform;
const export_Writable = ko.Writable;
const export_finished = Po.finished;
const export_pipeline = Oo.pipeline;
export {
	Jd as default,
	export_Duplex as Duplex,
	export_finished as finished,
	export_PassThrough as PassThrough,
	export_pipeline as pipeline,
	export_Readable as Readable,
	$ as Stream,
	export_Transform as Transform,
	export_Writable as Writable,
};
