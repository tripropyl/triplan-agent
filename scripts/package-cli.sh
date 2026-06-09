#!/usr/bin/env bash
set -euo pipefail

package="triplan-agent"
target="${1:-}"

if [[ -z "${target}" ]]; then
  target="$(rustc -vV | sed -n 's/^host: //p')"
fi

version="$(cargo pkgid -p "${package}" | sed 's/.*#//')"
dist_root="${DIST_DIR:-dist}"

if [[ "${target}" == "macos-universal" ]]; then
  artifact="${package}-${version}-macos-universal"
  package_dir="${dist_root}/${artifact}"

  cargo build --release -p "${package}" --target x86_64-apple-darwin
  cargo build --release -p "${package}" --target aarch64-apple-darwin

  rm -rf "${package_dir}"
  mkdir -p "${package_dir}"
  lipo -create \
    "target/x86_64-apple-darwin/release/${package}" \
    "target/aarch64-apple-darwin/release/${package}" \
    -output "${package_dir}/${package}"
  cp README.md "${package_dir}/"
  tar -C "${dist_root}" -czf "${dist_root}/${artifact}.tar.gz" "${artifact}"
  echo "${dist_root}/${artifact}.tar.gz"
  exit 0
fi

case "${target}" in
  x86_64-unknown-linux-gnu) label="linux-x64"; ext=""; archive="tar.gz" ;;
  aarch64-unknown-linux-gnu) label="linux-arm64"; ext=""; archive="tar.gz" ;;
  x86_64-pc-windows-msvc) label="windows-x64"; ext=".exe"; archive="zip" ;;
  aarch64-pc-windows-msvc) label="windows-arm64"; ext=".exe"; archive="zip" ;;
  x86_64-apple-darwin) label="macos-x64"; ext=""; archive="tar.gz" ;;
  aarch64-apple-darwin) label="macos-arm64"; ext=""; archive="tar.gz" ;;
  *) label="${target}"; ext=""; archive="tar.gz" ;;
esac

artifact="${package}-${version}-${label}"
package_dir="${dist_root}/${artifact}"
binary="target/${target}/release/${package}${ext}"

cargo build --release -p "${package}" --target "${target}"

rm -rf "${package_dir}"
mkdir -p "${package_dir}"
cp "${binary}" "${package_dir}/"
cp README.md "${package_dir}/"

if [[ "${archive}" == "zip" ]]; then
  (cd "${dist_root}" && zip -qr "${artifact}.zip" "${artifact}")
  echo "${dist_root}/${artifact}.zip"
else
  tar -C "${dist_root}" -czf "${dist_root}/${artifact}.tar.gz" "${artifact}"
  echo "${dist_root}/${artifact}.tar.gz"
fi
