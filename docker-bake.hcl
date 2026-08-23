variable "GIT_SHA" {
  default = "dev"
}

variable "RELEASE_TAG" {
  default = "dev"
}

variable "BUILD_CREATED" {
  default = formatdate("YYYY-MM-DD'T'hh:mm:ss'Z'", timestamp())
}

variable "API_REPO" {
  default = "harbor.k8s.khoobyar.name/joekhoobyar/autofile-api"
}

variable "UI_REPO" {
  default = "harbor.k8s.khoobyar.name/joekhoobyar/autofile-ui"
}

target "_common" {
  context   = "."
  platforms = ["linux/amd64", "linux/arm64"]
}

target "autofile-api" {
  inherits   = ["_common"]
  dockerfile = "api/Dockerfile"
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

group "default" {
  targets = ["adhoc"]
}
