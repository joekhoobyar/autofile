variable "GIT_SHA" {
  default = "dev"
}

variable "RELEASE_TAG" {
  default = "dev"
}

variable "BUILD_CREATED" {
  default = formatdate("YYYY-MM-DD'T'hh:mm:ss'Z'", timestamp())
}

variable "BASE_TAG" {
  default = "latest"
}

variable "API_REPO" {
  default = "ghcr.io/joekhoobyar/autofile-api"
}

variable "API_RUST_BASE_REPO" {
  default = "ghcr.io/joekhoobyar/autofile-api-rust-base"
}

variable "API_RUNTIME_BASE_REPO" {
  default = "ghcr.io/joekhoobyar/autofile-api-runtime-base"
}

variable "UI_REPO" {
  default = "ghcr.io/joekhoobyar/autofile-ui"
}

target "_common" {
  context   = "."
  platforms = ["linux/amd64", "linux/arm64"]
}

target "autofile-api-rust-base" {
  inherits   = ["_common"]
  dockerfile = "api/Dockerfile.rust-base"
  tags = [
    "${API_RUST_BASE_REPO}:${BASE_TAG}"
  ]
  labels = {
    "org.opencontainers.image.created"  = BUILD_CREATED
    "org.opencontainers.image.revision" = GIT_SHA
    "org.opencontainers.image.version"  = BASE_TAG
    "org.opencontainers.image.ref.name" = BASE_TAG
  }
  cache-from = [
    "type=registry,ref=${API_RUST_BASE_REPO}:buildcache"
  ]
  cache-to = [
    "type=registry,ref=${API_RUST_BASE_REPO}:buildcache,mode=max"
  ]
}

target "autofile-api-runtime-base" {
  inherits   = ["_common"]
  dockerfile = "api/Dockerfile.runtime-base"
  tags = [
    "${API_RUNTIME_BASE_REPO}:${BASE_TAG}"
  ]
  labels = {
    "org.opencontainers.image.created"  = BUILD_CREATED
    "org.opencontainers.image.revision" = GIT_SHA
    "org.opencontainers.image.version"  = BASE_TAG
    "org.opencontainers.image.ref.name" = BASE_TAG
  }
  cache-from = [
    "type=registry,ref=${API_RUNTIME_BASE_REPO}:buildcache"
  ]
  cache-to = [
    "type=registry,ref=${API_RUNTIME_BASE_REPO}:buildcache,mode=max"
  ]
}

target "autofile-api" {
  inherits   = ["_common"]
  dockerfile = "api/Dockerfile"
  args = {
    RUST_BASE_IMAGE        = "${API_RUST_BASE_REPO}:${BASE_TAG}"
    API_RUNTIME_BASE_IMAGE = "${API_RUNTIME_BASE_REPO}:${BASE_TAG}"
  }
  cache-from = [
    "type=registry,ref=${API_REPO}:buildcache"
  ]
  cache-to = [
    "type=registry,ref=${API_REPO}:buildcache,mode=max"
  ]
}

target "autofile-api-release" {
  inherits = ["autofile-api"]
  tags = [
    "${API_REPO}:latest",
    "${API_REPO}:${RELEASE_TAG}",
    "${API_REPO}:${GIT_SHA}"
  ]
  labels = {
    "org.opencontainers.image.created"  = BUILD_CREATED
    "org.opencontainers.image.revision" = GIT_SHA
    "org.opencontainers.image.version"  = RELEASE_TAG
    "org.opencontainers.image.ref.name" = RELEASE_TAG
  }
}

target "autofile-api-adhoc" {
  inherits = ["autofile-api"]
  tags = [
    "${API_REPO}:${GIT_SHA}"
  ]
  labels = {
    "org.opencontainers.image.created"  = BUILD_CREATED
    "org.opencontainers.image.revision" = GIT_SHA
    "org.opencontainers.image.version"  = GIT_SHA
    "org.opencontainers.image.ref.name" = GIT_SHA
  }
}

target "autofile-ui" {
  inherits   = ["_common"]
  dockerfile = "ui/Dockerfile"
  cache-from = [
    "type=registry,ref=${UI_REPO}:buildcache"
  ]
  cache-to = [
    "type=registry,ref=${UI_REPO}:buildcache,mode=max"
  ]
}

target "autofile-ui-release" {
  inherits = ["autofile-ui"]
  tags = [
    "${UI_REPO}:latest",
    "${UI_REPO}:${RELEASE_TAG}",
    "${UI_REPO}:${GIT_SHA}"
  ]
  labels = {
    "org.opencontainers.image.created"  = BUILD_CREATED
    "org.opencontainers.image.revision" = GIT_SHA
    "org.opencontainers.image.version"  = RELEASE_TAG
    "org.opencontainers.image.ref.name" = RELEASE_TAG
  }
}

target "autofile-ui-adhoc" {
  inherits = ["autofile-ui"]
  tags = [
    "${UI_REPO}:${GIT_SHA}"
  ]
  labels = {
    "org.opencontainers.image.created"  = BUILD_CREATED
    "org.opencontainers.image.revision" = GIT_SHA
    "org.opencontainers.image.version"  = GIT_SHA
    "org.opencontainers.image.ref.name" = GIT_SHA
  }
}

group "release" {
  targets = ["autofile-api-release", "autofile-ui-release"]
}

group "adhoc" {
  targets = ["autofile-api-adhoc", "autofile-ui-adhoc"]
}

group "base" {
  targets = ["autofile-api-rust-base", "autofile-api-runtime-base"]
}

group "default" {
  targets = ["adhoc"]
}
