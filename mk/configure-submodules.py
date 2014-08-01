# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

import subprocess
import os

def should_enable_debug(enable_debug, enable_debug_skia, submodule):
    if submodule in ("support/skia/skia", "support/azure/rust-azure"):
        # Right now the skia configure script actually ignores --enable-debug and the
        # build looks only at CFG_ENABLE_DEBUG_SKIA exported from our Makefile.  But we
        # still refrain from passing --enable-debug if we didn't get --enable-debug-skia,
        # in order to be more future-proof.
        #
        # The same applies to rust-azure below.  Also note that the two libraries won't
        # link if one is built with debugging and the other isn't.
        return enable_debug_skia

    if submodule == "support/phf/rust-phf":
        return False

    return enable_debug


def configure_submodules(src_dir, build_dir, submodules,
                         ostype, enable_debug, enable_debug_skia, extra_args,
                         android_cross_path, android_resource_path,
                         android_font_path, android_ndk_path):
    for submodule in submodules:
        #path = os.path.join(build_dir, "src", submodule)
        #if os.path.isdir(path):
        #    cd path
#${CFG_ANDROID_CROSS_PATH}
#${CFG_OSTYPE}
        m = {
            "platform/android/libexpat": {
                "path": os.path.join(src_dir, "src", submodule, "expat", "configure"),
                "args": [
                    "--host=arm-linux-androideabi",
                    "--with-sysroot=%s/sysroot" % android_cross_path,
                ] + extra_args,
            },
            "platform/android/libfreetype2": {
                "args": [
                    "--host=arm-linux",
                    "--with-sysroot=%s/sysroot" % android_cross_path,
                    "--without-zlib",
                ] + extra_args,
            },
            "platform/linux/fontconfig": {
                "path": os.path.join(src_dir, "src", submodule, "autogen.sh"),
                "args": [
                    "--sysconfdir=/etc",
                    "--localstatedir=/var",
                    "--disable-docs",
                    "--disable-shared", # work around Rust #12557
                ] + (
                    # Some RedHat-based distros (including our CentOS 6 build machines) are missing
                    # pkg-config files for expat: https://bugzilla.redhat.com/show_bug.cgi?id=833338
                    ["--with-expat=/usr"] if os.path.isfile("/etc/redhat-release") else []
                ) + extra_args,
            },
            "platform/android/fontconfig": {
                "path": os.path.join(src_dir, "src", submodule, "autogen.sh"),
                "args": [
                    "--host=arm-linux-androideabi",
                    "--with-arch=arm",
                    "--with-expat-includes=%ssrc/platform/android/libexpat/expat/lib" % src_dir,
                    "--with-expat-lib=%ssrc/platform/android/libexpat/.libs" % build_dir,
                    "--with-sysroot=%s/sysroot" % android_cross_path,
                    "--with-cache-dir=%s/.fccache" % android_resource_path,
                    "--with-confdir=%s/.fcconfig" % android_resource_path,
                    "--with-default-fonts=%s" % android_font_path,
                ] + extra_args,
            },
            "support/spidermonkey/mozjs": {
                "path": "${CFG_SRC_DIR}src/${i}/js/src/configure",
                "args": ([
                    "--target=arm-linux-androideabi",
                    "--with-android-ndk=%s" % android_ndk_path,
                    "--with-android-toolchain=%s" % android_cross_path,
                ] if ostype == "linux-androideabi" else []
                ) + [
                    "--enable-gczeal"
                ]
            },
            "support/azure/rust-azure": {
                "args": [
                    "--enable-skia",
                ]
            },
            "support/url/rust-url": {
                "path": os.path.join(src_dir, "src", "support", "url", "configure"),
            },
        }

        configure_path = m.get(submodule, {}).get("path", os.path.join(src_dir, "src", submodule, "configure"))
        configure_args = m.get(submodule, {}).get("args", [])

        if should_enable_debug(enable_debug, enable_debug_skia, submodule):
            configure_args += ["--enable-debug"]

        if os.path.isfile(configure_path):
            subprocess.check_call([configure_path] + configure_args)

def parse_args(args):
    import argparse
    parser = argparse.ArgumentParser(description='...')
    parser.add_argument('submodules', nargs='+',
                        help='the submodules')
    parser.add_argument('--srcdir', dest='src_dir',
                       help='sum the integers (default: find the max)')
    parser.add_argument('--builddir', dest='build_dir',
                       help='sum the integers (default: find the max)')
    parser.add_argument('--ostype')
    parser.add_argument('--enable-debug', dest='enable_debug')
    parser.add_argument('--enable-debug-skia', dest='enable_debug_skia')
    parser.add_argument('--extra-args', default=[])
    parser.add_argument('--android-cross-path')
    parser.add_argument('--android-resource-path')
    parser.add_argument('--android-font-path')
    parser.add_argument('--android-ndk-path')
    return vars(parser.parse_args(args))

if __name__ == "__main__":
    import sys
    try:
        configure_submodules(**parse_args(sys.argv))
    except subprocess.CalledProcessError as e:
        sys.exit(e.returncode)
