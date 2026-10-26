package provider

import (
	"context"
	"fmt"

	"github.com/hashicorp/terraform-plugin-framework/datasource"
	"github.com/hashicorp/terraform-plugin-framework/datasource/schema"
	"github.com/hashicorp/terraform-plugin-framework/types"
)

var (
	_ datasource.DataSource              = &targetDataSource{}
	_ datasource.DataSourceWithConfigure = &targetDataSource{}
	_ datasource.DataSource              = &channelDataSource{}
	_ datasource.DataSourceWithConfigure = &channelDataSource{}
)

func newTargetDataSource() datasource.DataSource  { return &targetDataSource{} }
func newChannelDataSource() datasource.DataSource { return &channelDataSource{} }

// --- dumbmonit_target ------------------------------------------------------

type targetDataSource struct{ client *apiClient }

type targetDataModel struct {
	ID           types.String `tfsdk:"id"`
	Name         types.String `tfsdk:"name"`
	Kind         types.String `tfsdk:"kind"`
	Address      types.String `tfsdk:"address"`
	IntervalSecs types.Int64  `tfsdk:"interval_secs"`
	Enabled      types.Bool   `tfsdk:"enabled"`
	GroupName    types.String `tfsdk:"group_name"`
}

func (d *targetDataSource) Metadata(_ context.Context, req datasource.MetadataRequest, resp *datasource.MetadataResponse) {
	resp.TypeName = req.ProviderTypeName + "_target"
}

func (d *targetDataSource) Schema(_ context.Context, _ datasource.SchemaRequest, resp *datasource.SchemaResponse) {
	resp.Schema = schema.Schema{
		Description: "Looks up an existing monitor by name, or by kind and address.",
		Attributes: map[string]schema.Attribute{
			"id":            schema.StringAttribute{Computed: true},
			"name":          schema.StringAttribute{Optional: true, Computed: true},
			"kind":          schema.StringAttribute{Optional: true, Computed: true},
			"address":       schema.StringAttribute{Optional: true, Computed: true},
			"interval_secs": schema.Int64Attribute{Computed: true},
			"enabled":       schema.BoolAttribute{Computed: true},
			"group_name":    schema.StringAttribute{Computed: true},
		},
	}
}

func (d *targetDataSource) Configure(_ context.Context, req datasource.ConfigureRequest, resp *datasource.ConfigureResponse) {
	d.client = clientFrom(req.ProviderData, &resp.Diagnostics)
}

func (d *targetDataSource) Read(ctx context.Context, req datasource.ReadRequest, resp *datasource.ReadResponse) {
	var cfg targetDataModel
	resp.Diagnostics.Append(req.Config.Get(ctx, &cfg)...)
	if resp.Diagnostics.HasError() {
		return
	}
	if cfg.Name.IsNull() && (cfg.Kind.IsNull() || cfg.Address.IsNull()) {
		resp.Diagnostics.AddError("Nothing to look up", "Set name, or both kind and address.")
		return
	}
	var all []targetAPI
	if err := d.client.do(ctx, "GET", "/api/targets", nil, &all); err != nil {
		resp.Diagnostics.AddError("Reading the monitors failed", err.Error())
		return
	}
	var found []targetAPI
	for _, target := range all {
		byName := !cfg.Name.IsNull() && target.Name == cfg.Name.ValueString()
		byAddress := !cfg.Kind.IsNull() && !cfg.Address.IsNull() &&
			target.Kind == cfg.Kind.ValueString() && target.Address == cfg.Address.ValueString()
		if byName || byAddress {
			found = append(found, target)
		}
	}
	if len(found) != 1 {
		resp.Diagnostics.AddError("Monitor not found or ambiguous", fmt.Sprintf("%d monitors match.", len(found)))
		return
	}
	api := found[0]
	cfg.ID = idString(api.ID)
	cfg.Name = types.StringValue(api.Name)
	cfg.Kind = types.StringValue(api.Kind)
	cfg.Address = types.StringValue(api.Address)
	cfg.IntervalSecs = types.Int64Value(api.IntervalSecs)
	cfg.Enabled = types.BoolValue(api.Enabled)
	cfg.GroupName = types.StringValue(api.GroupName)
	resp.Diagnostics.Append(resp.State.Set(ctx, &cfg)...)
}

// --- dumbmonit_notification_channel ---------------------------------------

type channelDataSource struct{ client *apiClient }

type channelDataModel struct {
	ID      types.String `tfsdk:"id"`
	Name    types.String `tfsdk:"name"`
	Kind    types.String `tfsdk:"kind"`
	Enabled types.Bool   `tfsdk:"enabled"`
}

func (d *channelDataSource) Metadata(_ context.Context, req datasource.MetadataRequest, resp *datasource.MetadataResponse) {
	resp.TypeName = req.ProviderTypeName + "_notification_channel"
}

func (d *channelDataSource) Schema(_ context.Context, _ datasource.SchemaRequest, resp *datasource.SchemaResponse) {
	resp.Schema = schema.Schema{
		Description: "Looks up an existing notification channel by name.",
		Attributes: map[string]schema.Attribute{
			"id":      schema.StringAttribute{Computed: true},
			"name":    schema.StringAttribute{Required: true},
			"kind":    schema.StringAttribute{Computed: true},
			"enabled": schema.BoolAttribute{Computed: true},
		},
	}
}

func (d *channelDataSource) Configure(_ context.Context, req datasource.ConfigureRequest, resp *datasource.ConfigureResponse) {
	d.client = clientFrom(req.ProviderData, &resp.Diagnostics)
}

func (d *channelDataSource) Read(ctx context.Context, req datasource.ReadRequest, resp *datasource.ReadResponse) {
	var cfg channelDataModel
	resp.Diagnostics.Append(req.Config.Get(ctx, &cfg)...)
	if resp.Diagnostics.HasError() {
		return
	}
	var all []channelAPI
	if err := d.client.do(ctx, "GET", "/api/notify/channels", nil, &all); err != nil {
		resp.Diagnostics.AddError("Reading the channels failed", err.Error())
		return
	}
	for _, channel := range all {
		if channel.Name == cfg.Name.ValueString() {
			cfg.ID = idString(channel.ID)
			cfg.Kind = types.StringValue(channel.Kind)
			cfg.Enabled = types.BoolValue(channel.Enabled)
			resp.Diagnostics.Append(resp.State.Set(ctx, &cfg)...)
			return
		}
	}
	resp.Diagnostics.AddError("Channel not found", fmt.Sprintf("No notification channel is named %q.", cfg.Name.ValueString()))
}
