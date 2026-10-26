package provider

import (
	"context"
	"errors"

	"github.com/hashicorp/terraform-plugin-framework/diag"
	"github.com/hashicorp/terraform-plugin-framework/path"
	"github.com/hashicorp/terraform-plugin-framework/resource"
	"github.com/hashicorp/terraform-plugin-framework/resource/schema"
	"github.com/hashicorp/terraform-plugin-framework/resource/schema/booldefault"
	"github.com/hashicorp/terraform-plugin-framework/resource/schema/int64planmodifier"
	"github.com/hashicorp/terraform-plugin-framework/resource/schema/planmodifier"
	"github.com/hashicorp/terraform-plugin-framework/resource/schema/stringdefault"
	"github.com/hashicorp/terraform-plugin-framework/resource/schema/stringplanmodifier"
	"github.com/hashicorp/terraform-plugin-framework/types"
)

var (
	_ resource.Resource                = &targetResource{}
	_ resource.ResourceWithConfigure   = &targetResource{}
	_ resource.ResourceWithImportState = &targetResource{}
)

func newTargetResource() resource.Resource { return &targetResource{} }

type targetResource struct{ client *apiClient }

type targetModel struct {
	ID             types.String `tfsdk:"id"`
	Name           types.String `tfsdk:"name"`
	Address        types.String `tfsdk:"address"`
	Kind           types.String `tfsdk:"kind"`
	IntervalSecs   types.Int64  `tfsdk:"interval_secs"`
	Enabled        types.Bool   `tfsdk:"enabled"`
	GroupName      types.String `tfsdk:"group_name"`
	Tags           types.Map    `tfsdk:"tags"`
	CredentialJSON types.String `tfsdk:"credential_json"`
}

// targetAPI : ce que l'API rend d'un équipement.
type targetAPI struct {
	ID           int64             `json:"id"`
	Name         string            `json:"name"`
	Address      string            `json:"address"`
	Kind         string            `json:"kind"`
	IntervalSecs int64             `json:"interval_secs"`
	Enabled      bool              `json:"enabled"`
	GroupName    string            `json:"group_name"`
	Tags         map[string]string `json:"tags"`
}

func (r *targetResource) Metadata(_ context.Context, req resource.MetadataRequest, resp *resource.MetadataResponse) {
	resp.TypeName = req.ProviderTypeName + "_target"
}

func (r *targetResource) Schema(_ context.Context, _ resource.SchemaRequest, resp *resource.SchemaResponse) {
	resp.Schema = schema.Schema{
		Description: "A monitored device or service.",
		Attributes: map[string]schema.Attribute{
			"id": schema.StringAttribute{
				Computed:      true,
				PlanModifiers: []planmodifier.String{stringplanmodifier.UseStateForUnknown()},
			},
			"name":    schema.StringAttribute{Required: true, Description: "Display name."},
			"address": schema.StringAttribute{Required: true, Description: "Host, URL or address, depending on the kind."},
			"kind": schema.StringAttribute{
				Required:      true,
				Description:   "Collector kind, as listed by GET /api/collectors (http, ping, snmp, proxmox...).",
				PlanModifiers: []planmodifier.String{stringplanmodifier.RequiresReplace()},
			},
			"interval_secs": schema.Int64Attribute{
				Optional:      true,
				Computed:      true,
				Description:   "Seconds between two checks (10 or more). The server default applies when omitted.",
				PlanModifiers: []planmodifier.Int64{int64planmodifier.UseStateForUnknown()},
			},
			"enabled": schema.BoolAttribute{
				Optional: true, Computed: true, Default: booldefault.StaticBool(true),
				Description: "False pauses the monitor.",
			},
			"group_name": schema.StringAttribute{
				Optional: true, Computed: true, Default: stringdefault.StaticString(""),
				Description: "Folder shown on the targets page. Empty: no folder.",
			},
			"tags": schema.MapAttribute{ElementType: types.StringType, Optional: true},
			"credential_json": schema.StringAttribute{
				Optional:  true,
				Sensitive: true,
				Description: "Credential as a JSON object, e.g. jsonencode({type = \"snmp_community\", community = var.community}). " +
					"The server never returns secrets, so changes made outside Terraform are not detected.",
			},
		},
	}
}

func (r *targetResource) Configure(_ context.Context, req resource.ConfigureRequest, resp *resource.ConfigureResponse) {
	r.client = clientFrom(req.ProviderData, &resp.Diagnostics)
}

func (r *targetResource) body(ctx context.Context, plan *targetModel, diags *diag.Diagnostics) map[string]any {
	body := map[string]any{
		"name":       plan.Name.ValueString(),
		"address":    plan.Address.ValueString(),
		"kind":       plan.Kind.ValueString(),
		"enabled":    plan.Enabled.ValueBool(),
		"group_name": plan.GroupName.ValueString(),
		"tags":       stringMap(ctx, plan.Tags, diags),
	}
	if !plan.IntervalSecs.IsNull() && !plan.IntervalSecs.IsUnknown() {
		body["interval_secs"] = plan.IntervalSecs.ValueInt64()
	}
	if credential := jsonAttr(plan.CredentialJSON, "credential_json", diags); credential != nil {
		body["credential"] = credential
	}
	return body
}

func fillTarget(model *targetModel, api targetAPI) {
	model.ID = idString(api.ID)
	model.Name = types.StringValue(api.Name)
	model.Address = types.StringValue(api.Address)
	model.Kind = types.StringValue(api.Kind)
	model.IntervalSecs = types.Int64Value(api.IntervalSecs)
	model.Enabled = types.BoolValue(api.Enabled)
	model.GroupName = types.StringValue(api.GroupName)
}

func (r *targetResource) Create(ctx context.Context, req resource.CreateRequest, resp *resource.CreateResponse) {
	var plan targetModel
	resp.Diagnostics.Append(req.Plan.Get(ctx, &plan)...)
	if resp.Diagnostics.HasError() {
		return
	}
	body := r.body(ctx, &plan, &resp.Diagnostics)
	if resp.Diagnostics.HasError() {
		return
	}
	var api targetAPI
	if err := r.client.do(ctx, "POST", "/api/targets", body, &api); err != nil {
		resp.Diagnostics.AddError("Creating the monitor failed", err.Error())
		return
	}
	fillTarget(&plan, api)
	resp.Diagnostics.Append(resp.State.Set(ctx, &plan)...)
}

func (r *targetResource) Read(ctx context.Context, req resource.ReadRequest, resp *resource.ReadResponse) {
	var state targetModel
	resp.Diagnostics.Append(req.State.Get(ctx, &state)...)
	if resp.Diagnostics.HasError() {
		return
	}
	var api targetAPI
	err := r.client.do(ctx, "GET", "/api/targets/"+state.ID.ValueString(), nil, &api)
	if errors.Is(err, errNotFound) {
		resp.State.RemoveResource(ctx)
		return
	}
	if err != nil {
		resp.Diagnostics.AddError("Reading the monitor failed", err.Error())
		return
	}
	fillTarget(&state, api)
	state.Tags = mapValue(ctx, api.Tags, state.Tags.IsNull(), &resp.Diagnostics)
	resp.Diagnostics.Append(resp.State.Set(ctx, &state)...)
}

func (r *targetResource) Update(ctx context.Context, req resource.UpdateRequest, resp *resource.UpdateResponse) {
	var plan, state targetModel
	resp.Diagnostics.Append(req.Plan.Get(ctx, &plan)...)
	resp.Diagnostics.Append(req.State.Get(ctx, &state)...)
	if resp.Diagnostics.HasError() {
		return
	}
	body := r.body(ctx, &plan, &resp.Diagnostics)
	if resp.Diagnostics.HasError() {
		return
	}
	var api targetAPI
	if err := r.client.do(ctx, "PUT", "/api/targets/"+state.ID.ValueString(), body, &api); err != nil {
		resp.Diagnostics.AddError("Updating the monitor failed", err.Error())
		return
	}
	fillTarget(&plan, api)
	resp.Diagnostics.Append(resp.State.Set(ctx, &plan)...)
}

func (r *targetResource) Delete(ctx context.Context, req resource.DeleteRequest, resp *resource.DeleteResponse) {
	var state targetModel
	resp.Diagnostics.Append(req.State.Get(ctx, &state)...)
	if resp.Diagnostics.HasError() {
		return
	}
	err := r.client.do(ctx, "DELETE", "/api/targets/"+state.ID.ValueString(), nil, nil)
	if err != nil && !errors.Is(err, errNotFound) {
		resp.Diagnostics.AddError("Deleting the monitor failed", err.Error())
	}
}

func (r *targetResource) ImportState(ctx context.Context, req resource.ImportStateRequest, resp *resource.ImportStateResponse) {
	resource.ImportStatePassthroughID(ctx, path.Root("id"), req, resp)
}
