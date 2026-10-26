package provider

import (
	"context"
	"errors"

	"github.com/hashicorp/terraform-plugin-framework/path"
	"github.com/hashicorp/terraform-plugin-framework/resource"
	"github.com/hashicorp/terraform-plugin-framework/resource/schema"
	"github.com/hashicorp/terraform-plugin-framework/resource/schema/planmodifier"
	"github.com/hashicorp/terraform-plugin-framework/resource/schema/stringplanmodifier"
	"github.com/hashicorp/terraform-plugin-framework/types"
)

var (
	_ resource.Resource                = &statusPageResource{}
	_ resource.ResourceWithConfigure   = &statusPageResource{}
	_ resource.ResourceWithImportState = &statusPageResource{}
)

func newStatusPageResource() resource.Resource { return &statusPageResource{} }

type statusPageResource struct{ client *apiClient }

type statusItemModel struct {
	TargetID  types.Int64  `tfsdk:"target_id"`
	Label     types.String `tfsdk:"label"`
	GroupName types.String `tfsdk:"group_name"`
}

type statusPageModel struct {
	ID            types.String      `tfsdk:"id"`
	Title         types.String      `tfsdk:"title"`
	Slug          types.String      `tfsdk:"slug"`
	Description   types.String      `tfsdk:"description"`
	Published     types.Bool        `tfsdk:"published"`
	Theme         types.String      `tfsdk:"theme"`
	ShowUptimeDay types.Int64       `tfsdk:"show_uptime_days"`
	Items         []statusItemModel `tfsdk:"items"`
}

type statusPageAPI struct {
	ID            int64  `json:"id"`
	Title         string `json:"title"`
	Slug          string `json:"slug"`
	Description   string `json:"description"`
	Published     bool   `json:"published"`
	Theme         string `json:"theme"`
	ShowUptimeDay int64  `json:"show_uptime_days"`
}

func (r *statusPageResource) Metadata(_ context.Context, req resource.MetadataRequest, resp *resource.MetadataResponse) {
	resp.TypeName = req.ProviderTypeName + "_status_page"
}

func (r *statusPageResource) Schema(_ context.Context, _ resource.SchemaRequest, resp *resource.SchemaResponse) {
	keep := []planmodifier.String{stringplanmodifier.UseStateForUnknown()}
	resp.Schema = schema.Schema{
		Description: "A public status page and the services it shows.",
		Attributes: map[string]schema.Attribute{
			"id":    schema.StringAttribute{Computed: true, PlanModifiers: keep},
			"title": schema.StringAttribute{Required: true},
			"slug": schema.StringAttribute{
				Optional: true, Computed: true,
				PlanModifiers: keep,
				Description:   "Address of the page (/status/<slug>); derived from the title when omitted.",
			},
			"description":      schema.StringAttribute{Optional: true, Computed: true, PlanModifiers: keep},
			"published":        schema.BoolAttribute{Optional: true, Computed: true},
			"theme":            schema.StringAttribute{Optional: true, Computed: true, PlanModifiers: keep},
			"show_uptime_days": schema.Int64Attribute{Optional: true, Computed: true},
			"items": schema.ListNestedAttribute{
				Optional:    true,
				Description: "Services shown, in display order.",
				NestedObject: schema.NestedAttributeObject{
					Attributes: map[string]schema.Attribute{
						"target_id":  schema.Int64Attribute{Required: true, Description: "Id of a dumbmonit_target."},
						"label":      schema.StringAttribute{Optional: true, Description: "Public name; the monitor name when empty."},
						"group_name": schema.StringAttribute{Optional: true, Description: "Heading the service is listed under."},
					},
				},
			},
		},
	}
}

func (r *statusPageResource) Configure(_ context.Context, req resource.ConfigureRequest, resp *resource.ConfigureResponse) {
	r.client = clientFrom(req.ProviderData, &resp.Diagnostics)
}

func (r *statusPageResource) pageBody(plan *statusPageModel) map[string]any {
	body := map[string]any{"title": plan.Title.ValueString()}
	set := func(key string, value types.String) {
		if !value.IsNull() && !value.IsUnknown() {
			body[key] = value.ValueString()
		}
	}
	set("slug", plan.Slug)
	set("description", plan.Description)
	set("theme", plan.Theme)
	if !plan.Published.IsNull() && !plan.Published.IsUnknown() {
		body["published"] = plan.Published.ValueBool()
	}
	if !plan.ShowUptimeDay.IsNull() && !plan.ShowUptimeDay.IsUnknown() {
		body["show_uptime_days"] = plan.ShowUptimeDay.ValueInt64()
	}
	return body
}

func (r *statusPageResource) putItems(ctx context.Context, id string, items []statusItemModel) error {
	payload := make([]map[string]any, 0, len(items))
	for _, item := range items {
		entry := map[string]any{"target_id": item.TargetID.ValueInt64()}
		if !item.Label.IsNull() {
			entry["label"] = item.Label.ValueString()
		}
		if !item.GroupName.IsNull() {
			entry["group_name"] = item.GroupName.ValueString()
		}
		payload = append(payload, entry)
	}
	return r.client.do(ctx, "PUT", "/api/status-pages/"+id+"/items", payload, nil)
}

func fillStatusPage(model *statusPageModel, api statusPageAPI) {
	model.ID = idString(api.ID)
	model.Slug = types.StringValue(api.Slug)
	model.Description = types.StringValue(api.Description)
	model.Published = types.BoolValue(api.Published)
	model.Theme = types.StringValue(api.Theme)
	model.ShowUptimeDay = types.Int64Value(api.ShowUptimeDay)
}

func (r *statusPageResource) Create(ctx context.Context, req resource.CreateRequest, resp *resource.CreateResponse) {
	var plan statusPageModel
	resp.Diagnostics.Append(req.Plan.Get(ctx, &plan)...)
	if resp.Diagnostics.HasError() {
		return
	}
	var api statusPageAPI
	if err := r.client.do(ctx, "POST", "/api/status-pages", r.pageBody(&plan), &api); err != nil {
		resp.Diagnostics.AddError("Creating the status page failed", err.Error())
		return
	}
	fillStatusPage(&plan, api)
	if plan.Items != nil {
		if err := r.putItems(ctx, plan.ID.ValueString(), plan.Items); err != nil {
			resp.Diagnostics.AddError("Setting the services of the status page failed", err.Error())
		}
	}
	resp.Diagnostics.Append(resp.State.Set(ctx, &plan)...)
}

func (r *statusPageResource) Read(ctx context.Context, req resource.ReadRequest, resp *resource.ReadResponse) {
	var state statusPageModel
	resp.Diagnostics.Append(req.State.Get(ctx, &state)...)
	if resp.Diagnostics.HasError() {
		return
	}
	var api statusPageAPI
	err := r.client.do(ctx, "GET", "/api/status-pages/"+state.ID.ValueString(), nil, &api)
	if errors.Is(err, errNotFound) {
		resp.State.RemoveResource(ctx)
		return
	}
	if err != nil {
		resp.Diagnostics.AddError("Reading the status page failed", err.Error())
		return
	}
	fillStatusPage(&state, api)
	state.Title = types.StringValue(api.Title)
	resp.Diagnostics.Append(resp.State.Set(ctx, &state)...)
}

func (r *statusPageResource) Update(ctx context.Context, req resource.UpdateRequest, resp *resource.UpdateResponse) {
	var plan, state statusPageModel
	resp.Diagnostics.Append(req.Plan.Get(ctx, &plan)...)
	resp.Diagnostics.Append(req.State.Get(ctx, &state)...)
	if resp.Diagnostics.HasError() {
		return
	}
	var api statusPageAPI
	if err := r.client.do(ctx, "PUT", "/api/status-pages/"+state.ID.ValueString(), r.pageBody(&plan), &api); err != nil {
		resp.Diagnostics.AddError("Updating the status page failed", err.Error())
		return
	}
	fillStatusPage(&plan, api)
	if plan.Items != nil {
		if err := r.putItems(ctx, plan.ID.ValueString(), plan.Items); err != nil {
			resp.Diagnostics.AddError("Setting the services of the status page failed", err.Error())
		}
	}
	resp.Diagnostics.Append(resp.State.Set(ctx, &plan)...)
}

func (r *statusPageResource) Delete(ctx context.Context, req resource.DeleteRequest, resp *resource.DeleteResponse) {
	var state statusPageModel
	resp.Diagnostics.Append(req.State.Get(ctx, &state)...)
	if resp.Diagnostics.HasError() {
		return
	}
	err := r.client.do(ctx, "DELETE", "/api/status-pages/"+state.ID.ValueString(), nil, nil)
	if err != nil && !errors.Is(err, errNotFound) {
		resp.Diagnostics.AddError("Deleting the status page failed", err.Error())
	}
}

func (r *statusPageResource) ImportState(ctx context.Context, req resource.ImportStateRequest, resp *resource.ImportStateResponse) {
	resource.ImportStatePassthroughID(ctx, path.Root("id"), req, resp)
}
