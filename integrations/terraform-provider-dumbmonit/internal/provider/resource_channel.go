package provider

import (
	"context"
	"encoding/json"
	"errors"

	"github.com/hashicorp/terraform-plugin-framework/path"
	"github.com/hashicorp/terraform-plugin-framework/resource"
	"github.com/hashicorp/terraform-plugin-framework/resource/schema"
	"github.com/hashicorp/terraform-plugin-framework/resource/schema/booldefault"
	"github.com/hashicorp/terraform-plugin-framework/resource/schema/planmodifier"
	"github.com/hashicorp/terraform-plugin-framework/resource/schema/stringplanmodifier"
	"github.com/hashicorp/terraform-plugin-framework/types"
)

var (
	_ resource.Resource                = &channelResource{}
	_ resource.ResourceWithConfigure   = &channelResource{}
	_ resource.ResourceWithImportState = &channelResource{}
)

func newChannelResource() resource.Resource { return &channelResource{} }

type channelResource struct{ client *apiClient }

type channelModel struct {
	ID           types.String `tfsdk:"id"`
	Name         types.String `tfsdk:"name"`
	Kind         types.String `tfsdk:"kind"`
	Enabled      types.Bool   `tfsdk:"enabled"`
	SettingsJSON types.String `tfsdk:"settings_json"`
	SecretsJSON  types.String `tfsdk:"secrets_json"`
	PolicyJSON   types.String `tfsdk:"policy_json"`
}

type channelAPI struct {
	ID      int64  `json:"id"`
	Name    string `json:"name"`
	Kind    string `json:"kind"`
	Enabled bool   `json:"enabled"`
}

func (r *channelResource) Metadata(_ context.Context, req resource.MetadataRequest, resp *resource.MetadataResponse) {
	resp.TypeName = req.ProviderTypeName + "_notification_channel"
}

func (r *channelResource) Schema(_ context.Context, _ resource.SchemaRequest, resp *resource.SchemaResponse) {
	resp.Schema = schema.Schema{
		Description: "A notification channel (ntfy, email, Slack, webhook...). Kinds and fields: GET /api/notify/kinds.",
		Attributes: map[string]schema.Attribute{
			"id": schema.StringAttribute{
				Computed:      true,
				PlanModifiers: []planmodifier.String{stringplanmodifier.UseStateForUnknown()},
			},
			"name": schema.StringAttribute{Required: true},
			"kind": schema.StringAttribute{
				Required:      true,
				PlanModifiers: []planmodifier.String{stringplanmodifier.RequiresReplace()},
			},
			"enabled": schema.BoolAttribute{Optional: true, Computed: true, Default: booldefault.StaticBool(true)},
			"settings_json": schema.StringAttribute{
				Optional:    true,
				Description: "Non-secret settings as a JSON object (jsonencode).",
			},
			"secrets_json": schema.StringAttribute{
				Optional:    true,
				Sensitive:   true,
				Description: "Secrets (tokens, passwords) as a JSON object. Never returned by the server, so drift is not detected.",
			},
			"policy_json": schema.StringAttribute{
				Optional:    true,
				Description: "Delivery policy of the channel as a JSON object (optional).",
			},
		},
	}
}

func (r *channelResource) Configure(_ context.Context, req resource.ConfigureRequest, resp *resource.ConfigureResponse) {
	r.client = clientFrom(req.ProviderData, &resp.Diagnostics)
}

func (r *channelResource) save(ctx context.Context, method, path string, plan *channelModel, diagsAdd func(string, string)) (channelAPI, bool) {
	body := map[string]any{
		"name":    plan.Name.ValueString(),
		"kind":    plan.Kind.ValueString(),
		"enabled": plan.Enabled.ValueBool(),
	}
	for key, value := range map[string]types.String{
		"settings": plan.SettingsJSON, "secrets": plan.SecretsJSON, "policy": plan.PolicyJSON,
	} {
		if value.IsNull() || value.IsUnknown() || value.ValueString() == "" {
			continue
		}
		var decoded any
		if err := json.Unmarshal([]byte(value.ValueString()), &decoded); err != nil {
			diagsAdd("Invalid JSON in "+key+"_json", err.Error())
			return channelAPI{}, false
		}
		body[key] = decoded
	}
	var api channelAPI
	if err := r.client.do(ctx, method, path, body, &api); err != nil {
		diagsAdd("Saving the channel failed", err.Error())
		return api, false
	}
	return api, true
}

func (r *channelResource) Create(ctx context.Context, req resource.CreateRequest, resp *resource.CreateResponse) {
	var plan channelModel
	resp.Diagnostics.Append(req.Plan.Get(ctx, &plan)...)
	if resp.Diagnostics.HasError() {
		return
	}
	api, ok := r.save(ctx, "POST", "/api/notify/channels", &plan, resp.Diagnostics.AddError)
	if !ok {
		return
	}
	plan.ID = idString(api.ID)
	resp.Diagnostics.Append(resp.State.Set(ctx, &plan)...)
}

func (r *channelResource) Read(ctx context.Context, req resource.ReadRequest, resp *resource.ReadResponse) {
	var state channelModel
	resp.Diagnostics.Append(req.State.Get(ctx, &state)...)
	if resp.Diagnostics.HasError() {
		return
	}
	var all []channelAPI
	if err := r.client.do(ctx, "GET", "/api/notify/channels", nil, &all); err != nil {
		resp.Diagnostics.AddError("Reading the channels failed", err.Error())
		return
	}
	want := parseID(state.ID, &resp.Diagnostics)
	for _, channel := range all {
		if channel.ID == want {
			state.Name = types.StringValue(channel.Name)
			state.Kind = types.StringValue(channel.Kind)
			state.Enabled = types.BoolValue(channel.Enabled)
			resp.Diagnostics.Append(resp.State.Set(ctx, &state)...)
			return
		}
	}
	resp.State.RemoveResource(ctx)
}

func (r *channelResource) Update(ctx context.Context, req resource.UpdateRequest, resp *resource.UpdateResponse) {
	var plan, state channelModel
	resp.Diagnostics.Append(req.Plan.Get(ctx, &plan)...)
	resp.Diagnostics.Append(req.State.Get(ctx, &state)...)
	if resp.Diagnostics.HasError() {
		return
	}
	if _, ok := r.save(ctx, "PUT", "/api/notify/channels/"+state.ID.ValueString(), &plan, resp.Diagnostics.AddError); !ok {
		return
	}
	plan.ID = state.ID
	resp.Diagnostics.Append(resp.State.Set(ctx, &plan)...)
}

func (r *channelResource) Delete(ctx context.Context, req resource.DeleteRequest, resp *resource.DeleteResponse) {
	var state channelModel
	resp.Diagnostics.Append(req.State.Get(ctx, &state)...)
	if resp.Diagnostics.HasError() {
		return
	}
	err := r.client.do(ctx, "DELETE", "/api/notify/channels/"+state.ID.ValueString(), nil, nil)
	if err != nil && !errors.Is(err, errNotFound) {
		resp.Diagnostics.AddError("Deleting the channel failed", err.Error())
	}
}

func (r *channelResource) ImportState(ctx context.Context, req resource.ImportStateRequest, resp *resource.ImportStateResponse) {
	resource.ImportStatePassthroughID(ctx, path.Root("id"), req, resp)
}
