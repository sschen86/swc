//// [awaitUsingDeclarations.2.ts]
import { _ as _async_to_generator } from "@swc/helpers/_/_async_to_generator";
import { _ as _dispose } from "@swc/helpers/_/_dispose";
import { _ as _using } from "@swc/helpers/_/_using";
try {
    var _stack = [];
    _using(_stack, {
        [Symbol.asyncDispose]: ()=>_async_to_generator(function*() {})()
    }, !0), _using(_stack, {
        [Symbol.asyncDispose]: ()=>_async_to_generator(function*() {})()
    }, !0);
} catch (_) {
    var _error = _, _hasError = !0;
} finally{
    await _dispose(_stack, _error, _hasError);
}
