define([
    "require",
    "exports",
    "foo"
], function(require, exports, _foo) {
    "use strict";
    Object.defineProperty(exports, "__esModule", {
        value: true
    });
    Object.defineProperty(exports, "foo", {
        get: ()=>_foo.default,
        enumerable: true
    });
    _foo = _interopRequireDefault(_foo);
});
