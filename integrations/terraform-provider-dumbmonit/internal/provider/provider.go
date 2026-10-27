// Package provider implémente le fournisseur Terraform de DumbMonit.
package provider

import (
	"context"
	"os"

	"github.com/hashicorp/terraform-plugin-framework/datasource"
	"github.com/hashicorp/terraform-plugin-framework/path"
	"github.com/hashicorp/terraform-plugin-framework/provider"
	"github.com/hashicorp/terraform-plugin-framework/provider/schema"
	"github.com/hashicorp/terraform-plugin-framework/resource"
	"github.com/hashicorp/terraform-plugin-framework/types"
)

var _ provider.Provider = &dumbmonitProvider{}

type dumbmonitProvider struct {
	version string
}

// New construit le fournisseur.
func New(version string) func() provider.Provider {
	return func() provider.Provider { return &dumbmonitProvider{version: version} }
}

type providerModel struct {
	URL      types.String `tfsdk:"url"`
	Token    types.String `tfsdk:"token"`
	Insecure types.Bool   `tfsdk:"insecure"`
}

func (p *dumbmonitProvider) Metadata(_ context.Context, _ provider.MetadataRequest, resp *provider.MetadataResponse) {
	resp.TypeName = "dumbmonit"
	resp.Version = p.version
}

func (p *dumbmonitProvider) Schema(_ context.Context, _ provider.SchemaRequest, resp *provider.SchemaResponse) {
	resp.Schema = schema.Schema{
		Description: "Manage a DumbMonit instance: monitors, notification channels, alert rules, maintenance windows and status pages.",
		Attributes: map[string]schema.Attribute{
			"url": schema.StringAttribute{
				Optional:    true,
				Description: "Address of the server, e.g. https://monit.example.org. Defaults to the DUMBMONIT_URL environment variable.",
			},
			"token": schema.StringAttribute{
				Optional:    true,
				Sensitive:   true,
				Description: "Administrator API token with write scope. Defaults to the DUMBMONIT_TOKEN environment variable.",
			},
			"insecure": schema.BoolAttribute{
				Optional:    true,
				Description: "Do not verify the server certificate (self-signed lab instances only).",
			},
		},
	}
}

func (p *dumbmonitProvider) Configure(ctx context.Context, req provider.ConfigureRequest, resp *provider.ConfigureResponse) {
	var cfg providerModel
	resp.Diagnostics.Append(req.Config.Get(ctx, &cfg)...)
	if resp.Diagnostics.HasError() {
		return
	}
	url := cfg.URL.ValueString()
	if url == "" {
		url = os.Getenv("DUMBMONIT_URL")
	}
	token := cfg.Token.ValueString()
	if token == "" {
		token = os.Getenv("DUMBMONIT_TOKEN")
	}
	if url == "" {
		resp.Diagnostics.AddAttributeError(path.Root("url"), "Missing server address",
			"Set the url attribute or the DUMBMONIT_URL environment variable.")
	}
	if token == "" {
		resp.Diagnostics.AddAttributeError(path.Root("token"), "Missing API token",
			"Set the token attribute or the DUMBMONIT_TOKEN environment variable.")
	}
	if resp.Diagnostics.HasError() {
		return
	}
	client := newClient(url, token, cfg.Insecure.ValueBool())
	resp.ResourceData = client
	resp.DataSourceData = client
}

func (p *dumbmonitProvider) Resources(_ context.Context) []func() resource.Resource {
	return []func() resource.Resource{
		newTargetResource,
		newChannelResource,
		newRuleResource,
		newSilenceResource,
		newStatusPageResource,
	}
}

func (p *dumbmonitProvider) DataSources(_ context.Context) []func() datasource.DataSource {
	return []func() datasource.DataSource{
		newTargetDataSource,
		newChannelDataSource,
	}
}
