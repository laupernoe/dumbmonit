terraform {
  required_providers {
    dumbmonit = { source = "registry.terraform.io/selfhostr/dumbmonit" }
  }
}

provider "dumbmonit" {
  # url and token come from DUMBMONIT_URL and DUMBMONIT_TOKEN.
}

resource "dumbmonit_target" "website" {
  name    = "Website"
  kind    = "http"
  address = "https://example.org"
}
