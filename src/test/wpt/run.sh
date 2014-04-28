# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

set -e

servo_root="$1"
objdir="$2"
shift 2

cd $objdir/..
test -d _virtualenv || virtualenv _virtualenv
test -d $servo_root/src/test/wpt/metadata || mkdir -p $servo_root/src/test/wpt/metadata
test -d $servo_root/src/test/wpt/prefs || mkdir -p $servo_root/src/test/wpt/prefs
source _virtualenv/bin/activate
(python -c "import html5lib" &>/dev/null) || pip install html5lib
(python -c "import wptrunner"  &>/dev/null) || pip install wptrunner

python $servo_root/src/test/wpt/run.py --binary $objdir/../servo --log-mach - "$@"
