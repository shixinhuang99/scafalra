set -e

readonly build="npm run build"
readonly test="npm run test"
readonly publish="npm publish"
readonly push="git push"
readonly pushTags="git push --tags"

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
${push}
${pushTags}
