set -e

readonly build="npm run build"
readonly test="npm run test"
readonly publish="npm publish"

function check {
  if [ $? -ne 0 ]; then
    exit 0
  fi
}

${build}
check
${test}
check
${publish}
