"use strict";
Object.defineProperty(exports, "__esModule", {
    value: true
});
Object.defineProperty(exports, "default", {
    get: ()=>Foo,
    enumerable: true
});
const _asyncToGenerator = require("@swc/helpers/lib/_async_to_generator.js").default;
const _interopRequireDefault = require("@swc/helpers/lib/_interop_require_default.js").default;
const _react = _interopRequireDefault(require("react"));
function Foo() {
    return /*#__PURE__*/ _react.default.createElement("div", {
        onClick: _asyncToGenerator(function*(e) {
            yield doSomething();
        })
    });
}
Foo.displayName = "Foo";
