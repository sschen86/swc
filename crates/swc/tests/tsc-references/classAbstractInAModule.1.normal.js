//// [classAbstractInAModule.ts]
import { _ as _class_call_check } from "@swc/helpers/_/_class_call_check";
import { _ as _inherits } from "@swc/helpers/_/_inherits";
import { _ as _create_super } from "@swc/helpers/_/_create_super";
var M;
(function(M) {
    var A = function A() {
        "use strict";
        _class_call_check(this, A);
    };
    M.A = A;
    var B = /*#__PURE__*/ function(A) {
        "use strict";
        _inherits(B, A);
        var _super = _create_super(B);
        function B() {
            _class_call_check(this, B);
            return _super.apply(this, arguments);
        }
        return B;
    }(A);
    M.B = B;
})(M || (M = {}));
new M.A;
new M.B;
