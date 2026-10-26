package provider

import (
	"context"
	"errors"

	"github.com/hashicorp/terraform-plugin-framework/resource"
	"github.com/hashicorp/terraform-plugin-framework/resource/schema"
	"github.com/hashicorp/terraform-plugin-framework/resource/schema/int64planmodifier"
	"github.com/hashicorp/terraform-plugin-framework/resource/schema/planmodifier"
	"github.com/hashicorp/terraform-plugin-framework/resource/schema/stringplanmodifier"
	"github.com/hashicorp/terraform-plugin-framework/types"
)

var (
	_ resource.Resource              = &silenceResource{}
	_ resource.ResourceWithConfigure = &silenceResource{}
)

func newSilenceResource() resource.Resource { return &silenceResource{} }

type silenceResource struct{ client *apiClient }

type silenceModel struct {
	ID       types.String `tfsdk:"id"`
	Name     types.String `tfsdk:"name"`
	Comment  types.String `tfsdk:"comment"`
	TargetID types.Int64  `tfsdk:"target_id"`
	Matchers types.Map    `tfsdk:"matchers"`
	StartsAt types.String `tfsdk:"starts_at"`
	EndsAt   types.String `tfsdk:"ends_at"`
}

func (r *silenceResource) Metadata(_ context.Context, req resource.MetadataRequest, resp *resource.MetadataResponse) {
	resp.TypeName = req.ProviderTypeName + "_silence"
}

func (r *silenceResource) Schema(_ context.Context, _ resource.SchemaRequest, resp *resource.SchemaResponse) {
	replace := []planmodifier.String{stringplanmodifier.RequiresReplace()}
	resp.Schema = schema.Schema{
		Description: "A one-off maintenance window. Silences are immutable: any change replaces the window.",
		Attributes: map[string]schema.Attribute{
			"id":      schema.StringAttribute{Computed: true, PlanModifiers: []planmodifier.String{stringplanmodifier.UseStateForUnknown()}},
			"name":    schema.StringAttribute{Required: true, PlanModifiers: replace},
			"comment": schema.StringAttribute{Optional: true, PlanModifiers: replace},
			"target_id": schema.Int64Attribute{
				Optional: true, PlanModifiers: []planmodifier.Int64{int64planmodifier.RequiresReplace()},
				Description: "Silence one monitor only (id of a dumbmonit_target).",
			},
			"matchers": schema.MapAttribute{
				ElementType: types.StringType, Optional: true,
				Description: "Label matchers narrowing what is silenced.",
			},
			"starts_at": schema.StringAttribute{Required: true, PlanModifiers: replace, Description: "RFC 3339, UTC."},
			"ends_at":   schema.StringAttribute{Required: true, PlanModifiers: replace, Description: "RFC 3339, UTC."},
		},
	}
}

func (r *silenceResource) Configure(_ context.Context, req resource.ConfigureRequest, resp *resource.ConfigureResponse) {
	r.client = clientFrom(req.ProviderData, &resp.Diagnostics)
}

func (r *silenceResource) Create(ctx context.Context, req resource.CreateRequest, resp *resource.CreateResponse) {
	var plan silenceModel
	resp.Diagnostics.Append(req.Plan.Get(ctx, &plan)...)
	if resp.Diagnostics.HasError() {
		return
	}
	body := map[string]any{
		"name":     plan.Name.ValueString(),
		"comment":  plan.Comment.ValueString(),
		"matchers": stringMap(ctx, plan.Matchers, &resp.Diagnostics),
		"schedule": map[string]any{"kind": "once", "starts_at": plan.StartsAt.ValueString(), "ends_at": plan.EndsAt.ValueString()},
		"enabled":  true,
	}
	if !plan.TargetID.IsNull() {
		body["target_id"] = plan.TargetID.ValueInt64()
	}
	var api struct {
		ID int64 `json:"id"`
	}
	if err := r.client.do(ctx, "POST", "/api/alerts/silences", body, &api); err != nil {
		resp.Diagnostics.AddError("Creating the maintenance window failed", err.Error())
		return
	}
	plan.ID = idString(api.ID)
	resp.Diagnostics.Append(resp.State.Set(ctx, &plan)...)
}

func (r *silenceResource) Read(ctx context.Context, req resource.ReadRequest, resp *resource.ReadResponse) {
	var state silenceModel
	resp.Diagnostics.Append(req.State.Get(ctx, &state)...)
	if resp.Diagnostics.HasError() {
		return
	}
	var all []struct {
		ID int64 `json:"id"`
	}
	if err := r.client.do(ctx, "GET", "/api/alerts/silences", nil, &all); err != nil {
		resp.Diagnostics.AddError("Reading the maintenance windows failed", err.Error())
		return
	}
	want := parseID(state.ID, &resp.Diagnostics)
	for _, silence := range all {
		if silence.ID == want {
			return // inchangé : la fenêtre existe toujours
		}
	}
	// Une fenêtre ponctuelle échue est retirée par le serveur : on l'oublie aussi.
	resp.State.RemoveResource(ctx)
}

func (r *silenceResource) Update(_ context.Context, _ resource.UpdateRequest, resp *resource.UpdateResponse) {
	resp.Diagnostics.AddError("Maintenance windows cannot be updated", "Every attribute forces a new window; this is a provider bug.")
}

func (r *silenceResource) Delete(ctx context.Context, req resource.DeleteRequest, resp *resource.DeleteResponse) {
	var state silenceModel
	resp.Diagnostics.Append(req.State.Get(ctx, &state)...)
	if resp.Diagnostics.HasError() {
		return
	}
	err := r.client.do(ctx, "DELETE", "/api/alerts/silences/"+state.ID.ValueString(), nil, nil)
	if err != nil && !errors.Is(err, errNotFound) {
		resp.Diagnostics.AddError("Deleting the maintenance window failed", err.Error())
	}
}
