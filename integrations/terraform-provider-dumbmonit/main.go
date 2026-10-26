// Fournisseur Terraform pour DumbMonit.
package main

import (
	"context"
	"flag"
	"log"

	"github.com/hashicorp/terraform-plugin-framework/providerserver"

	"github.com/laupernoe/dumbmonit/integrations/terraform-provider-dumbmonit/internal/provider"
)

// version est posée à la compilation (-ldflags "-X main.version=...").
var version = "dev"

func main() {
	var debug bool
	flag.BoolVar(&debug, "debug", false, "lance le fournisseur pour un débogueur")
	flag.Parse()

	err := providerserver.Serve(context.Background(), provider.New(version), providerserver.ServeOpts{
		Address: "registry.terraform.io/laupernoe/dumbmonit",
		Debug:   debug,
	})
	if err != nil {
		log.Fatal(err.Error())
	}
}
