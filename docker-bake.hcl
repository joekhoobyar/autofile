variable "GIT_SHA" {
  default = "dev"
}

variable "DATE_TAG" {
  default = formatdate("YYYYMMDD", timestamp())
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
  tags = [
    "${API_REPO}:latest",
    "${API_REPO}:${DATE_TAG}-${GIT_SHA}"
  ]
  cache-from = [
    "type=registry,ref=${API_REPO}:buildcache"
  ]
  cache-to = [
    "type=registry,ref=${API_REPO}:buildcache,mode=max"
  ]
}

target "autofile-ui" {
  inherits   = ["_common"]
  dockerfile = "ui/Dockerfile"
  tags = [
    "${UI_REPO}:latest",
    "${UI_REPO}:${DATE_TAG}-${GIT_SHA}"
  ]
  cache-from = [
    "type=registry,ref=${UI_REPO}:buildcache"
  ]
  cache-to = [
    "type=registry,ref=${UI_REPO}:buildcache,mode=max"
  ]
}

group "default" {
  targets = ["autofile-api", "autofile-ui"]
}
