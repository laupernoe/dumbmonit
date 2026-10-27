package provider

import (
	"context"
	"encoding/json"
	"errors"

	"github.com/hashicorp/terraform-plugin-framework/diag"
	"github.com/hashicorp/terraform-plugin-framework/path"
	"github.com/hashicorp/terraform-plugin-framework/resource"
	"github.com/hashicorp/terraform-plugin-framework/resource/schema"
	"github.com/hashicorp/terraform-plugin-framework/resource/schema/planmodifier"
	"github.com/hashicorp/terraform-plugin-framework/resource/schema/stringplanmodifier"
	"github.com/hashicorp/terraform-plugin-framework/types"
)

var (
	_ resource.Resource                = &ruleResource{}
	_ resource.ResourceWithConfigure   = &ruleResource{}
	_ resource.ResourceWithImportState = &ruleResource{}
)

func newRuleResource() resource.Resource { return &ruleResource{} }

type ruleResource struct{ client *apiClient }

type ruleModel struct {
	ID             types.String  `tfsdk:"id"`
	UID            types.String  `tfsdk:"uid"`
	Name           types.String  `tfsdk:"name"`
	Description    types.String  `tfsdk:"description"`
	Kind           types.String  `tfsdk:"kind"`
	Query          types.String  `tfsdk:"query"`
	Operator       types.String  `tfsdk:"operator"`
	Threshold      types.Float64 `tfsdk:"threshold"`
	ClearThreshold types.Float64 `tfsdk:"clear_threshold"`
	ForSecs        types.Int64   `tfsdk:"for_secs"`
	Severity       types.String  `tfsdk:"severity"`
	SelectorJSON   types.String  `tfsdk:"selector_json"`
	ChannelIDs     types.List    `tfsdk:"channel_ids"`
	RepeatSecs     types.Int64   `tfsdk:"repeat_secs"`
	Enabled        types.Bool    `tfsdk:"enabled"`
}

type ruleAPI struct {
	ID          int64   `json:"id"`
	UID         string  `json:"uid"`
	Name        string  `json:"name"`
	Description string  `json:"description"`
	Kind        string  `json:"kind"`
	Query       string  `json:"query"`
	Operator    string  `json:"operator"`
	Threshold   float64 `json:"threshold"`
	ForSecs     int64   `json:"for_secs"`
	Severity    string  `json:"severity"`
	Enabled     bool    `json:"enabled"`
}

func (r *ruleResource) Metadata(_ context.Context, req resource.MetadataRequest, resp *resource.MetadataResponse) {
	resp.TypeName = req.ProviderTypeName + "_alert_rule"
}

func (r *ruleResource) Schema(_ context.Context, _ resource.SchemaRequest, resp *resource.SchemaResponse) {
	keep := []planmodifier.String{stringplanmodifier.UseStateForUnknown()}
	resp.Schema = schema.Schema{
		Description: "An alert rule. Built-in rules are not managed here.",
		Attributes: map[string]schema.Attribute{
			"id":          schema.StringAttribute{Computed: true, PlanModifiers: keep},
			"uid":         schema.StringAttribute{Optional: true, Computed: true, PlanModifiers: []planmodifier.String{stringplanmodifier.UseStateForUnknown(), stringplanmodifier.RequiresReplace()}, Description: "Stable identifier; derived from the name when omitted."},
			"name":        schema.StringAttribute{Required: true},
			"description": schema.StringAttribute{Optional: true, Computed: true, PlanModifiers: keep},
			"kind":        schema.StringAttribute{Optional: true, Computed: true, PlanModifiers: keep, Description: "threshold (default) or anomaly."},
			"query":       schema.StringAttribute{Required: true, Description: "Metric query evaluated for each monitor."},
			"operator":    schema.StringAttribute{Optional: true, Computed: true, PlanModifiers: keep, Description: "Comparison operator: >, >=, < or <=."},
			"threshold":   schema.Float64Attribute{Optional: true, Computed: true},
			"clear_threshold": schema.Float64Attribute{
				Optional: true, Description: "Value at which the alert clears (hysteresis).",
			},
			"for_secs":      schema.Int64Attribute{Optional: true, Computed: true, Description: "How long the condition must hold."},
			"severity":      schema.StringAttribute{Optional: true, Computed: true, PlanModifiers: keep},
			"selector_json": schema.StringAttribute{Optional: true, Description: "Which monitors the rule applies to, as JSON (default: all)."},
			"channel_ids": schema.ListAttribute{
				ElementType: types.Int64Type, Optional: true,
				Description: "Notification channels to use; empty means every enabled channel.",
			},
			"repeat_secs": schema.Int64Attribute{Optional: true, Description: "Reminder period while the alert stays open."},
			"enabled":     schema.BoolAttribute{Optional: true, Computed: true},
		},
	}
}

func (r *ruleResource) Configure(_ context.Context, req resource.ConfigureRequest, resp *resource.ConfigureResponse) {
	r.client = clientFrom(req.ProviderData, &resp.Diagnostics)
}

func (r *ruleResource) body(ctx context.Context, plan *ruleModel, diags *diag.Diagnostics) map[string]any {
	body := map[string]any{"name": plan.Name.ValueString(), "query": plan.Query.ValueString()}
	set := func(key string, value types.String) {
		if !value.IsNull() && !value.IsUnknown() {
			body[key] = value.ValueString()
		}
	}
	set("uid", plan.UID)
	set("description", plan.Description)
	set("kind", plan.Kind)
	set("operator", plan.Operator)
	set("severity", plan.Severity)
	if !plan.Threshold.IsNull() && !plan.Threshold.IsUnknown() {
		body["threshold"] = plan.Threshold.ValueFloat64()
	}
	if !plan.ClearThreshold.IsNull() {
		body["clear_threshold"] = plan.ClearThreshold.ValueFloat64()
	}
	if !plan.ForSecs.IsNull() && !plan.ForSecs.IsUnknown() {
		body["for_secs"] = plan.ForSecs.ValueInt64()
	}
	if !plan.RepeatSecs.IsNull() {
		body["repeat_secs"] = plan.RepeatSecs.ValueInt64()
	}
	if !plan.Enabled.IsNull() && !plan.Enabled.IsUnknown() {
		body["enabled"] = plan.Enabled.ValueBool()
	}
	if selector := jsonAttr(plan.SelectorJSON, "selector_json", diags); selector != nil {
		body["selector"] = selector
	}
	if !plan.ChannelIDs.IsNull() && !plan.ChannelIDs.IsUnknown() {
		var ids []int64
		diags.Append(plan.ChannelIDs.ElementsAs(ctx, &ids, false)...)
		body["channels"] = ids
	}
	return body
}

func fillRule(model *ruleModel, api ruleAPI) {
	model.ID = idString(api.ID)
	model.UID = types.StringValue(api.UID)
	model.Description = types.StringValue(api.Description)
	model.Kind = types.StringValue(api.Kind)
	model.Operator = types.StringValue(api.Operator)
	model.Threshold = types.Float64Value(api.Threshold)
	model.ForSecs = types.Int64Value(api.ForSecs)
	model.Severity = types.StringValue(api.Severity)
	model.Enabled = types.BoolValue(api.Enabled)
}

func (r *ruleResource) Create(ctx context.Context, req resource.CreateRequest, resp *resource.CreateResponse) {
	var plan ruleModel
	resp.Diagnostics.Append(req.Plan.Get(ctx, &plan)...)
	if resp.Diagnostics.HasError() {
		return
	}
	body := r.body(ctx, &plan, &resp.Diagnostics)
	if resp.Diagnostics.HasError() {
		return
	}
	var api ruleAPI
	if err := r.client.do(ctx, "POST", "/api/alerts/rules", body, &api); err != nil {
		resp.Diagnostics.AddError("Creating the rule failed", err.Error())
		return
	}
	fillRule(&plan, api)
	resp.Diagnostics.Append(resp.State.Set(ctx, &plan)...)
}

func (r *ruleResource) Read(ctx context.Context, req resource.ReadRequest, resp *resource.ReadResponse) {
	var state ruleModel
	resp.Diagnostics.Append(req.State.Get(ctx, &state)...)
	if resp.Diagnostics.HasError() {
		return
	}
	raw := json.RawMessage{}
	if err := r.client.do(ctx, "GET", "/api/alerts/rules", nil, &raw); err != nil {
		resp.Diagnostics.AddError("Reading the rules failed", err.Error())
		return
	}
	var all []ruleAPI
	if err := json.Unmarshal(raw, &all); err != nil {
		resp.Diagnostics.AddError("Unexpected rule list", err.Error())
		return
	}
	want := parseID(state.ID, &resp.Diagnostics)
	for _, rule := range all {
		if rule.ID == want {
			fillRule(&state, rule)
			state.Name = types.StringValue(rule.Name)
			state.Query = types.StringValue(rule.Query)
			resp.Diagnostics.Append(resp.State.Set(ctx, &state)...)
			return
		}
	}
	resp.State.RemoveResource(ctx)
}

func (r *ruleResource) Update(ctx context.Context, req resource.UpdateRequest, resp *resource.UpdateResponse) {
	var plan, state ruleModel
	resp.Diagnostics.Append(req.Plan.Get(ctx, &plan)...)
	resp.Diagnostics.Append(req.State.Get(ctx, &state)...)
	if resp.Diagnostics.HasError() {
		return
	}
	body := r.body(ctx, &plan, &resp.Diagnostics)
	if resp.Diagnostics.HasError() {
		return
	}
	var api ruleAPI
	if err := r.client.do(ctx, "PUT", "/api/alerts/rules/"+state.ID.ValueString(), body, &api); err != nil {
		resp.Diagnostics.AddError("Updating the rule failed", err.Error())
		return
	}
	fillRule(&plan, api)
	resp.Diagnostics.Append(resp.State.Set(ctx, &plan)...)
}

func (r *ruleResource) Delete(ctx context.Context, req resource.DeleteRequest, resp *resource.DeleteResponse) {
	var state ruleModel
	resp.Diagnostics.Append(req.State.Get(ctx, &state)...)
	if resp.Diagnostics.HasError() {
		return
	}
	err := r.client.do(ctx, "DELETE", "/api/alerts/rules/"+state.ID.ValueString(), nil, nil)
	if err != nil && !errors.Is(err, errNotFound) {
		resp.Diagnostics.AddError("Deleting the rule failed", err.Error())
	}
}

func (r *ruleResource) ImportState(ctx context.Context, req resource.ImportStateRequest, resp *resource.ImportStateResponse) {
	resource.ImportStatePassthroughID(ctx, path.Root("id"), req, resp)
}
